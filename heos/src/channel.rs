//! Command/response channels.
//!
//! This module contains the implementations that manage sending [commands](crate::command) and
//! receiving [responses](crate::data::response). In general, the contents of this module are not
//! commonly used directly by users, as it is more ergonomic to use the higher level
//! [connection](crate::HeosConnection) abstractions.

use ahash::HashMap;
use async_trait::async_trait;
use educe::Educe;
use parking_lot::Mutex;
use std::fmt::Debug;
use std::io::Result as IoResult;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{tcp, TcpStream};
use tokio::sync::broadcast::{
    Receiver as BroadcastReceiver,
    Sender as BroadcastSender,
};
use tracing::{error, trace, warn};

use crate::command::raw::RawCommand;
use crate::command::{Command, CommandError};
use crate::data::event::Event;
use crate::data::response::RawResponse;

#[derive(Debug)]
struct DelayedResponse {
    waker: Option<Waker>,
    response: Option<RawResponse>,
}

/// Future for retrieving a [RawResponse] from the HEOS connection.
#[derive(Debug)]
struct RawResponseFuture {
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

/// Interface for the backend definition for a [Channel].
///
/// The backend is responsible for actually sending and receiving raw data. The implementation can
/// change for e.g. WASM vs other platforms, or for custom test implementations.
///
/// In general, the user should never need to implement their own backend, as the default
/// implementations used by this library should suffice. However, in the event that the user wants
/// to communicate with a HEOS system in a way this library doesn't support, they can do so via this
/// trait.
///
/// # Example Implementation
/// ```
/// use async_trait::async_trait;
/// use heos::channel::{ChannelBackend, ChannelState};
/// use heos::command::raw::RawCommand;
/// use parking_lot::Mutex;
/// use std::sync::Arc;
///
/// #[derive(Debug)]
/// struct MyChannelBackend {
///     state: Option<Arc<Mutex<ChannelState>>>,
/// }
///
/// #[async_trait]
/// impl ChannelBackend for MyChannelBackend {
///     async fn init(&mut self, state: Arc<Mutex<ChannelState>>) -> Result<(), std::io::Error> {
///         // Store the state. The implementation should have some method of processing incoming
///         // messages and giving them to this stored state.
///         self.state = Some(state);
///
///         // An implementation may want to start e.g. a background task or thread here.
///
///         Ok(())
///     }
///
///     async fn send(&mut self, command: RawCommand) -> Result<(), std::io::Error> {
///         // Get raw bytes for a command.
///         let bytes = command.to_string().as_bytes();
///
///         // Do something to send the command somewhere.
///
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait ChannelBackend: Debug + Send + Sync + 'static {
    /// Initialize this backend with [ChannelState].
    ///
    /// The [ChannelState] should be stored, and can be used to handle incoming messages.
    async fn init(&mut self, state: Arc<Mutex<ChannelState>>) -> IoResult<()>;
    /// Send a [RawCommand] message.
    async fn send(&mut self, command: RawCommand) -> IoResult<()>;
}

#[derive(Educe)]
#[educe(Debug, Default)]
struct ResponseCache {
    #[educe(Debug(ignore))]
    current: Option<tokio::sync::oneshot::Sender<Option<RawResponse>>>,
    delayed: HashMap<u64, Arc<Mutex<DelayedResponse>>>,
    last_delayed: Option<(u64, Arc<Mutex<DelayedResponse>>)>,
}

/// Channel state.
///
/// Most of the implementation of this state is internal, but users can use a mutable reference to
/// this state to handle incoming messages via [`ChannelState::handle_response()`].
#[derive(Educe)]
#[educe(Debug)]
pub struct ChannelState {
    response_caches: HashMap<String, ResponseCache>,
    event_broadcast: BroadcastSender<Event>,
}

impl Default for ChannelState {
    #[inline]
    fn default() -> Self {
        Self {
            response_caches: HashMap::default(),
            event_broadcast: BroadcastSender::new(Channel::EVENT_BROADCAST_BUFFER),
        }
    }
}

impl ChannelState {
    /// Handle an incoming message that has already been parsed into a [RawResponse].
    pub fn handle_response(&mut self, response: RawResponse) {

        if response.heos.message.starts_with("command under process") {
            trace!(?response, "Received delay response");
            if let Some(response_cache) = self.response_caches.get_mut(&response.heos.command) {
                if let Some(current_response) = response_cache.current.take() {
                    let _ = current_response.send(None);
                }
            }
        } else if response.heos.command.starts_with("event/") {
            let event = match Event::try_from(response) {
                Ok(event) => event,
                Err(error) => {
                    error!(?error, "Failed to parse incoming event");
                    return
                },
            };
            // We don't care if there are no receivers
            let _ = self.event_broadcast.send(event);
        } else {
            let response_cache = match self.response_caches.get_mut(&response.heos.command) {
                Some(response_cache) => response_cache,
                None => {
                    warn!(command = ?&response.heos.command, ?response, "Unexpected command response");
                    return
                },
            };

            if let Some(current_response) = response_cache.current.take() {
                let _ = current_response.send(Some(response));
                return
            }

            let maybe_msg_id = match response.try_msg_id() {
                Ok(maybe_msg_id) => maybe_msg_id,
                Err(error) => {
                    error!(?error, "Failed to parse incoming message ID");
                    return
                },
            };

            if let Some(msg_id) = maybe_msg_id {
                if let Some(delayed_response) = response_cache.delayed.remove(&msg_id) {
                    let mut delayed_response = delayed_response.lock();
                    delayed_response.response = Some(response);
                    if let Some(waker) = delayed_response.waker.take() {
                        waker.wake();
                    }
                    if let Some((cached_msg_id, _)) = &response_cache.last_delayed && *cached_msg_id == msg_id {
                        response_cache.last_delayed = None;
                    }
                } else {
                    warn!(?msg_id, ?response, "Unmatched response");
                }
            } else {
                if let Some((cached_msg_id, delayed_response)) = response_cache.last_delayed.take() {
                    let mut delayed_response = delayed_response.lock();
                    delayed_response.response = Some(response);
                    if let Some(waker) = delayed_response.waker.take() {
                        waker.wake();
                    }
                    response_cache.delayed.remove(&cached_msg_id);
                }
            }
        }
    }
}

#[derive(Debug)]
struct TcpRwPair {
    read_handle: tokio::task::JoinHandle<()>,
    writer: tcp::OwnedWriteHalf,
}

/// Channel backend used for TCP connections.
///
/// This allows connection to a HEOS system via a direct TCP socket.
#[derive(Debug)]
pub struct TcpChannel {
    socket_addr: SocketAddr,
    rw_pair: Option<TcpRwPair>,
}

impl TcpChannel {
    /// Create a new TCP channel.
    ///
    /// This method does not immediately connect to the given `socket_addr`, but will instead store
    /// it and attempt a connection when the backend is later [initialized](ChannelBackend::init).
    #[inline]
    pub fn new(socket_addr: SocketAddr) -> Self {
        Self {
            socket_addr,
            rw_pair: None,
        }
    }

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
}

#[async_trait]
impl ChannelBackend for TcpChannel {
    async fn init(&mut self, state: Arc<Mutex<ChannelState>>) -> IoResult<()> {
        let stream = TcpStream::connect(self.socket_addr).await?;
        let (reader, writer) = stream.into_split();

        if let Some(rw_pair) = self.rw_pair.take() {
            rw_pair.read_handle.abort();
        }

        let read_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            loop {
                let response = match Self::read_response(&mut reader).await {
                    Ok(response) => response,
                    Err(error) => {
                        error!(?error, "Failed to read incoming message");
                        continue
                    },
                };

                state.lock().handle_response(response);
            }
        });

        self.rw_pair = Some(TcpRwPair {
            read_handle,
            writer,
        });

        Ok(())
    }

    async fn send(&mut self, command: RawCommand) -> IoResult<()> {
        if let Some(rw_pair) = &mut self.rw_pair {
            rw_pair.writer.write_all(command.to_string().as_bytes()).await?;
            rw_pair.writer.write_all(b"\r\n").await?;
        }
        Ok(())
    }
}

impl Drop for TcpChannel {
    fn drop(&mut self) {
        if let Some(rw_pair) = self.rw_pair.take() {
            rw_pair.read_handle.abort();
        }
    }
}

/// Channel for sending [commands](crate::command) and receiving [responses](crate::data::response).
#[derive(Debug)]
pub struct Channel {
    backend: Box<dyn ChannelBackend>,
    next_msg_id: AtomicU64,
    state: Arc<Mutex<ChannelState>>,
}

impl Channel {
    /// How many [events](Event) can be held onto before they start being dropped without being
    /// processed.
    ///
    /// See [Self::subscribe_event_broadcast()] for more.
    pub const EVENT_BROADCAST_BUFFER: usize = 32;

    /// Create a new channel with the specified backend.
    pub async fn new(backend: impl ChannelBackend) -> IoResult<Self> {
        let mut backend: Box<dyn ChannelBackend> = Box::new(backend);
        let next_msg_id = AtomicU64::new(0);
        let state = Arc::new(Mutex::new(ChannelState::default()));

        backend.init(state.clone()).await?;

        Ok(Self {
            backend,
            next_msg_id,
            state,
        })
    }

    /// Send a [RawCommand] through this channel.
    ///
    /// This yields the [RawResponse] if successful.
    ///
    /// # Errors
    ///
    /// Errors if the backend has an [IO error](std::io::Error).
    pub async fn send_raw_command(&mut self, command: RawCommand) -> Result<RawResponse, std::io::Error> {
        let mut command = command;
        let msg_id = self.next_msg_id.fetch_add(1, Ordering::Relaxed);
        command.param("SEQUENCE", msg_id.to_string());
        let command_id = command.command();
        let command_str = command.to_string();

        let fut = RawResponseFuture::new();
        let (tx, rc) = tokio::sync::oneshot::channel();
        {
            let mut state = self.state.lock();
            let response_cache = state.response_caches.entry(command_id.clone()).or_default();
            response_cache.current = Some(tx);
            response_cache.delayed.insert(msg_id, fut.inner.clone());
            response_cache.last_delayed = Some((msg_id, fut.inner.clone()));
        }

        trace!(?command_str, "Sending command");
        self.backend.send(command).await?;

        let maybe_raw_response = rc.await
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;

        if let Some(raw_response) = maybe_raw_response {
            fut.inner.lock().response = Some(raw_response);
            {
                let mut state = self.state.lock();
                if let Some(response_cache) = state.response_caches.get_mut(&command_id) {
                    response_cache.delayed.remove(&msg_id);
                    if let Some((cached_msg_id, _)) = &response_cache.last_delayed && *cached_msg_id == msg_id {
                        response_cache.last_delayed = None;
                    }
                }
            }
        }

        Ok(fut.await)
    }

    /// Send a [Command] through this channel.
    ///
    /// This yields the [response type](Command::Response) associated with the command.
    ///
    /// # Errors
    ///
    /// Errors if the backend has an [IO error](std::io::Error), or if the [RawResponse] represents
    /// an execution error or fails to parse into the typed response.
    pub async fn send_command<C>(&mut self, command: C) -> Result<C::Response, CommandError>
    where
        C: Command
    {
        let raw_command = RawCommand::from_command(&command)?;
        let raw_response = self.send_raw_command(raw_command).await?;
        raw_response.validate_command()?;
        C::Response::try_from(raw_response)
    }

    /// Subscribe to [change events](crate::data::event) received by this channel.
    ///
    /// This uses a [tokio broadcast](tokio::sync::broadcast) implementation, and has the same
    /// restrictions. This means every subscriber needs to handle an event before it is dropped,
    /// otherwise they will accumulate in the internal buffer. If the buffer's
    /// [maximum capacity](Self::EVENT_BROADCAST_BUFFER) is exceeded, the oldest event will be
    /// dropped, and any subscribers which have not handled it will never receive it.
    #[inline]
    pub fn subscribe_event_broadcast(&self) -> BroadcastReceiver<Event> {
        self.state.lock().event_broadcast.subscribe()
    }
}