use ahash::HashMap;
use parking_lot::Mutex;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use educe::Educe;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{tcp, TcpStream};
use tokio::sync::broadcast::{
    Receiver as BroadcastReceiver,
    Sender as BroadcastSender,
};
use tracing::{error, trace, warn};

use crate::command::{Command, CommandError};
use crate::command::raw::RawCommand;
use crate::data::event::Event;
use crate::data::response::RawResponse;

#[derive(Debug)]
struct DelayedResponse {
    waker: Option<Waker>,
    response: Option<RawResponse>,
}

#[derive(Debug)]
pub struct RawResponseFuture {
    inner: Arc<Mutex<DelayedResponse>>,
}

impl RawResponseFuture {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DelayedResponse {
                waker: None,
                response: None,
            }))
        }
    }
}

impl Future for RawResponseFuture {
    type Output = RawResponse;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.inner.lock();
        if let Some(response) = inner.response.take() {
            Poll::Ready(response)
        } else {
            if let Some(waker) = inner.waker.as_mut() {
                waker.clone_from(cx.waker());
            } else {
                inner.waker = Some(cx.waker().clone());
            }
            Poll::Pending
        }
    }
}

#[derive(Educe)]
#[educe(Debug)]
struct ChannelState {
    current_response: Option<tokio::sync::oneshot::Sender<Option<RawResponse>>>,
    delayed_responses: HashMap<u64, Arc<Mutex<DelayedResponse>>>,
    event_broadcast: BroadcastSender<Event>,
}

#[derive(Debug)]
pub struct Channel {
    next_msg_id: AtomicU64,
    state: Arc<Mutex<ChannelState>>,
    read_handle: tokio::task::JoinHandle<()>,
    writer: tcp::OwnedWriteHalf,
}

impl Channel {
    pub(crate) const EVENT_BROADCAST_BUFFER: usize = 32;
    
    async fn read(reader: &mut BufReader<tcp::OwnedReadHalf>) -> Result<String, std::io::Error> {
        let mut buf = Vec::new();
        loop {
            reader.read_until(b'\n', &mut buf).await?;
            let len = buf.len();
            // Separator bytes are b'\r\n'
            if len >= 2 && buf[len - 2] == b'\r' {
                return match String::from_utf8(buf) {
                    Ok(msg) => {
                        trace!(?msg, "Received message");
                        Ok(msg)
                    },
                    Err(err) => Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Not valid UTF-8: '{err:?}'"),
                    )),
                }
            }
        }
    }

    async fn read_response(reader: &mut BufReader<tcp::OwnedReadHalf>) -> Result<RawResponse, std::io::Error> {
        let response_str = Self::read(reader).await?;
        serde_json::from_str(&response_str)
            .map_err(|err| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                err,
            ))
    }

    pub async fn connect(socket_addr: SocketAddr) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect(socket_addr).await?;
        let (reader, writer) = stream.into_split();

        let state = Arc::new(Mutex::new(ChannelState {
            current_response: None,
            delayed_responses: HashMap::default(),
            event_broadcast: BroadcastSender::new(Self::EVENT_BROADCAST_BUFFER),
        }));

        let read_handle = {
            let state = state.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(reader);
                loop {
                    let response = match Self::read_response(&mut reader).await {
                        Ok(response) => response,
                        Err(error) => {
                            error!(?error, "Failed to read incoming message");
                            continue
                        },
                    };

                    if response.heos.message.starts_with("command under process") {
                        trace!(?response, "Received delay response");
                        if let Some(current_response) = state.lock().current_response.take() {
                            let _ = current_response.send(None);
                        }
                        continue
                    } else if response.heos.command.starts_with("event/") {
                        let event = match Event::try_from(response) {
                            Ok(event) => event,
                            Err(error) => {
                                error!(?error, "Failed to parse incoming event");
                                continue
                            },
                        };
                        // We don't care if there are no receivers
                        let _ = state.lock().event_broadcast.send(event);
                    } else {
                        let mut state = state.lock();

                        if let Some(current_response) = state.current_response.take() {
                            let _ = current_response.send(Some(response));
                            continue
                        }

                        let maybe_msg_id = match response.try_msg_id() {
                            Ok(maybe_msg_id) => maybe_msg_id,
                            Err(error) => {
                                error!(?error, "Failed to parse incoming message ID");
                                continue
                            },
                        };

                        let msg_id = match maybe_msg_id {
                            Some(msg_id) => msg_id,
                            None => {
                                warn!(?response, "Unexpected unsolicited message");
                                continue
                            }
                        };

                        if let Some(delayed_response) = state.delayed_responses.remove(&msg_id) {
                            let mut delayed_response = delayed_response.lock();
                            delayed_response.response = Some(response);
                            if let Some(waker) = delayed_response.waker.take() {
                                waker.wake();
                            }
                        } else {
                            warn!(?msg_id, ?response, "Unmatched response");
                            continue
                        }
                    }
                }
            })
        };

        Ok(Self {
            next_msg_id: AtomicU64::new(0),
            state,
            read_handle,
            writer,
        })
    }

    async fn write(&mut self, bytes: impl AsRef<[u8]>) -> Result<(), std::io::Error> {
        self.writer.write_all(bytes.as_ref()).await?;
        self.writer.write_all(b"\r\n").await
    }

    pub async fn send_raw_command(&mut self, command: RawCommand) -> Result<RawResponseFuture, std::io::Error> {
        let mut command = command;
        let msg_id = self.next_msg_id.fetch_add(1, Ordering::Relaxed);
        command.param("SEQUENCE", msg_id.to_string());
        let command_str = command.to_string();

        let fut = RawResponseFuture::new();
        let (tx, rc) = tokio::sync::oneshot::channel();
        {
            let mut state = self.state.lock();
            state.current_response = Some(tx);
            state.delayed_responses.insert(msg_id, fut.inner.clone());
        }

        trace!(?command_str, "Sending command");
        self.write(command.to_string()).await?;

        let maybe_raw_response = rc.await
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;

        if let Some(raw_response) = maybe_raw_response {
            fut.inner.lock().response = Some(raw_response);
            self.state.lock().delayed_responses.remove(&msg_id);
        }

        Ok(fut)
    }

    pub async fn send_command<C>(&mut self, command: C) -> Result<C::Response, CommandError>
    where
        C: Command
    {
        let raw_command = RawCommand::from_command(&command)?;
        let raw_response = self.send_raw_command(raw_command).await?.await;
        raw_response.validate_command()?;
        C::Response::try_from(raw_response)
    }

    #[inline]
    pub fn subscribe_event_broadcast(&self) -> BroadcastReceiver<Event> {
        self.state.lock().event_broadcast.subscribe()
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        self.read_handle.abort();
    }
}