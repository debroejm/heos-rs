use ssdp_client::{SearchTarget, URN};
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{
    broadcast::Receiver as BroadcastReceiver,
    Mutex as AsyncMutex,
};
use tokio_stream::{Stream, StreamExt};
use tracing::{trace, warn};
use url::{Host, Url};

pub use ssdp_client::Error as ScanError;

use crate::channel::Channel;
use crate::command::raw::RawCommand;
use crate::command::system::RegisterForChangeEvents;
use crate::command::{Command, CommandError};
use crate::data::event::Event;
use crate::data::system::ChangeEventsEnabled;
use crate::state::State;

pub use channel::RawResponseFuture;

mod channel;
pub mod command;
pub mod data;
pub mod state;

#[derive(Debug)]
pub struct Created {
    ip: IpAddr,
}

#[derive(Debug)]
pub struct HeosConnection<S> {
    state: S,
}

#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    #[error("SSDP scan error: {0}")]
    ScanError(#[from] ScanError),
    #[error("No HEOS devices were found on the network")]
    NoDevicesFound,
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Command failed while initializing: {0}")]
    CommandError(#[from] CommandError),
}

impl HeosConnection<Created> {
    const HEOS_PORT: u16 = 1255;

    pub async fn scan(
        timeout: Duration,
    ) -> Result<impl Stream<Item=Self>, ScanError> {
        let search_target = SearchTarget::URN(URN::device(
            "schemas-denon-com",
            "ACT-Denon",
            1,
        ));

        let mx = 2.min(timeout.as_secs()).max(1) as usize;

        let responses = ssdp_client::search(
            &search_target,
            timeout,
            mx,
            None,
        ).await?;

        Ok(responses
            .filter_map(|result| match result {
                Ok(response) => {
                    trace!(?response, "Received SSDP response");
                    match Url::parse(response.location()) {
                        Ok(location) => {
                            let ip: Option<IpAddr> = match location.host() {
                                Some(host) => match host {
                                    Host::Ipv4(ip) => Some(ip.into()),
                                    Host::Ipv6(ip) => Some(ip.into()),
                                    host => {
                                        warn!(?location, ?host, "Unsupported host type");
                                        None
                                    }
                                },
                                None => {
                                    warn!(?location, "No host type found");
                                    None
                                },
                            };

                            ip.map(|ip| Self {
                                state: Created {
                                    ip,
                                }
                            })
                        },
                        Err(error) => {
                            warn!(?error, url = ?response.location(), "Could not parse device URL");
                            None
                        }
                    }
                },
                Err(error) => {
                    warn!(?error, "Failed search request");
                    None
                },
            }))
    }

    pub async fn connect(self) -> Result<HeosConnection<AdHoc>, ConnectError> {
        let socket_addr = SocketAddr::new(self.state.ip, Self::HEOS_PORT);
        let channel = AsyncMutex::new(Channel::connect(socket_addr).await?);

        let connection = HeosConnection {
            state: AdHoc {
                channel,
            }
        };

        connection.command(RegisterForChangeEvents {
            enable: ChangeEventsEnabled::Off,
        }).await?;

        Ok(connection)
    }

    pub async fn connect_any(
        timeout: Duration,
    ) -> Result<HeosConnection<AdHoc>, ConnectError> {
        Self::scan(timeout).await?
            .next().await.ok_or(ConnectError::NoDevicesFound)?
            .connect().await
    }
}

pub trait ConnectedState {
    fn channel(&self) -> &AsyncMutex<Channel>;
}

impl<S: ConnectedState> HeosConnection<S> {
    pub async fn raw_command(&self, command: RawCommand) -> Result<RawResponseFuture, std::io::Error> {
        self.state.channel().lock().await.send_raw_command(command).await
    }

    pub async fn command<C>(&self, command: C) -> Result<C::Response, CommandError>
    where
        C: Command,
    {
        self.state.channel().lock().await.send_command(command).await
    }

    pub async fn subscribe_event_broadcast(&self) -> BroadcastReceiver<Event> {
        self.state.channel().lock().await.subscribe_event_broadcast()
    }
}

#[derive(Debug)]
pub struct AdHoc {
    channel: AsyncMutex<Channel>,
}

impl ConnectedState for AdHoc {
    #[inline]
    fn channel(&self) -> &AsyncMutex<Channel> {
        &self.channel
    }
}

impl HeosConnection<AdHoc> {
    pub async fn init_stateful(self) -> Result<HeosConnection<Stateful>, CommandError> {
        let state = Arc::new(State::init(self.state.channel.into_inner()).await?);
        let event_handle = {
            let state = state.clone();
            let mut event_broadcast = state.channel.lock().await.subscribe_event_broadcast();
            tokio::spawn(async move {
                loop {
                    let event = match event_broadcast.recv().await {
                        Ok(event) => event,
                        Err(_) => break,
                    };

                    // TODO: Limit amount of spawned event handlers
                    let state = state.clone();
                    tokio::spawn(async move {
                        match state.handle_event(event).await {
                            Ok(_) => {},
                            Err(error) => {
                                warn!(?error, "Failed to handle event");
                            }
                        }
                    });
                }
            })
        };

        state.channel.lock().await
            .send_command(RegisterForChangeEvents {
                enable: ChangeEventsEnabled::On,
            }).await?;

        // TODO: Does the state need to be refreshed after registering for change events?
        //  Theoretically something could change between init and registering

        Ok(HeosConnection {
            state: Stateful {
                state,
                event_handle,
            },
        })
    }
}

#[derive(Debug)]
pub struct Stateful {
    state: Arc<State>,
    event_handle: tokio::task::JoinHandle<()>,
}

impl Drop for Stateful {
    fn drop(&mut self) {
        self.event_handle.abort();
    }
}

impl ConnectedState for Stateful {
    #[inline]
    fn channel(&self) -> &AsyncMutex<Channel> {
        &self.state.channel
    }
}

impl Deref for HeosConnection<Stateful> {
    type Target = State;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.state.state
    }
}