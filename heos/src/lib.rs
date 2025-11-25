//! Rust bindings for the HEOS control protocol.
//!
//! The published specifications for the latest version of the CLI (1.17 at time of writing) can be
//! found here:
//! https://rn.dmglobal.com/usmodel/HEOS_CLI_ProtocolSpecification-Version-1.17.pdf
//!
//! If that links gets stale and no longer works, a newer version may be able to be found on the
//! Denon support website, here:
//! https://support.denon.com/app/answers/detail/a_id/6953/~/heos-control-protocol-%28cli%29
//!
//! # Getting a Connection
//!
//! A HEOS system on the local network can be found via SSDP discovery. The following initiates SSDP
//! discovery and yields an asynchronous stream of possible HEOS connection endpoints as they're
//! discovered:
//!
//! ```
//! use heos::HeosConnection;
//! # use heos::{Created, ScanError};
//! use std::time::Duration;
//! # use tokio_stream::Stream;
//!
//! # async fn wrapper() -> Result<impl Stream<Item=HeosConnection<Created>>, ScanError> {
//! let endpoints = HeosConnection::scan(Duration::from_secs(10)).await?;
//! # Ok(endpoints)
//! # }
//! ```
//!
//! Once endpoints have been discovered, any of them can be chosen to be used as the connection. The
//! HEOS CLI uses a distributed system where a connection to any HEOS device can control all HEOS
//! devices on the same network.
//!
//! ```
//! use heos::{ConnectError, HeosConnection};
//! # use heos::AdHoc;
//! use std::time::Duration;
//! use tokio_stream::StreamExt;
//!
//! # async fn wrapper() -> Result<HeosConnection<AdHoc>, ConnectError> {
//! let mut endpoints = HeosConnection::scan(Duration::from_secs(10)).await?;
//! let connection = endpoints.next().await
//!     .ok_or(ConnectError::NoDevicesFound)?
//!     .connect().await?;
//! # Ok(connection)
//! # }
//! ```
//!
//! Or, to do all of the above in one method:
//!
//! ```
//! use heos::HeosConnection;
//! # use heos::{AdHoc, ConnectError};
//! use std::time::Duration;
//!
//! # async fn wrapper() -> Result<HeosConnection<AdHoc>, ConnectError> {
//! let connection = HeosConnection::connect_any(Duration::from_secs(10)).await?;
//! # Ok(connection)
//! # }
//! ```
//!
//! # Stateful Connections
//!
//! The HEOS system supports sending change events whenever any part of the internal state changes.
//! Using these, we can maintain a stateful representation of the system without needing to re-query
//! all the time.
//!
//! A stateful connection can be initiated like so:
//!
//! ```
//! use heos::HeosConnection;
//! # use heos::{Stateful, ConnectError};
//! use std::time::Duration;
//!
//! # async fn wrapper() -> Result<HeosConnection<Stateful>, ConnectError> {
//! let connection = HeosConnection::connect_any(Duration::from_secs(10)).await?;
//! let stateful = connection.init_stateful().await?;
//! # Ok(stateful)
//! # }
//! ```

use ssdp_client::{SearchTarget, URN};
use std::net::{IpAddr, SocketAddr};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{
    broadcast::Receiver as BroadcastReceiver,
    broadcast::Sender as BroadcastSender,
    Mutex as AsyncMutex,
    MutexGuard as AsyncMutexGuard,
};
use tokio_stream::{Stream, StreamExt};
use tracing::{trace, warn};
use url::{Host, Url};

pub use ssdp_client::Error as ScanError;

use crate::channel::{Channel, TcpChannel};
use crate::command::raw::RawCommand;
use crate::command::system::RegisterForChangeEvents;
use crate::command::{Command, CommandError};
use crate::data::event::Event;
use crate::data::response::RawResponse;
use crate::data::system::ChangeEventsEnabled;
use crate::doctest::try_doctest_channel;
use crate::state::State;

pub mod channel;
pub mod command;
pub mod data;
mod doctest;
pub mod mock;
pub mod state;

#[doc(hidden)]
pub use doctest::install_doctest_handler;

/// Inner state for a [HeosConnection] object that has been created but not connected.
///
/// Connections of this type represent a valid IP endpoint as determined by an SSDP scan, but no
/// attempt to actually connect has been made yet.
#[derive(Debug)]
pub struct Created {
    ip: IpAddr,
}

/// Main connection object of the library.
///
/// A HeosConnection is the centralized object where all other operations stem from. This object
/// can be in several states, depending on how far along the connection process is.
#[derive(Debug)]
pub struct HeosConnection<S> {
    state: S,
}

/// Errors that can occur when connecting a [HeosConnection].
#[derive(thiserror::Error, Debug)]
pub enum ConnectError {
    /// There was an error while scanning for valid endpoints to connect to.
    #[error("SSDP scan error: {0}")]
    ScanError(#[from] ScanError),
    /// There are no valid HEOS devices on the local network to connect to.
    #[error("No HEOS devices were found on the network")]
    NoDevicesFound,
    /// Some other IO error occurred.
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    /// An error occurred when sending commands during initialization.
    #[error("Command failed while initializing: {0}")]
    CommandError(#[from] CommandError),
}

impl HeosConnection<Created> {
    const HEOS_PORT: u16 = 1255;

    /// Perform a SSDP scan on a local network to find valid HEOS endpoints to connect to.
    ///
    /// Note that this method does not attempt to connect to any endpoints; it only discovers them.
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

    /// Connect to the endpoint currently represented by this HeosConnection.
    ///
    /// This will transition the internal state from [Created] to [AdHoc].
    pub async fn connect(self) -> Result<HeosConnection<AdHoc>, ConnectError> {
        let socket_addr = SocketAddr::new(self.state.ip, Self::HEOS_PORT);
        let channel = Channel::new(TcpChannel::new(socket_addr)).await?;

        let connection = HeosConnection::from_channel(channel).await?;

        Ok(connection)
    }

    /// Connect to any valid HEOS endpoint on the local network.
    pub async fn connect_any(
        timeout: Duration,
    ) -> Result<HeosConnection<AdHoc>, ConnectError> {
        if let Some(doctest_channel) = try_doctest_channel() {
            Ok(HeosConnection::from_channel(Channel::new(doctest_channel).await?).await?)
        } else {
            Self::scan(timeout).await?
                .next().await.ok_or(ConnectError::NoDevicesFound)?
                .connect().await
        }
    }

    /// The IP address of this possible connection.
    #[inline]
    pub fn ip(&self) -> IpAddr {
        self.state.ip
    }
}

trait ConnectedState {
    fn channel(&self) -> &AsyncMutex<Channel>;
}

#[allow(private_bounds)]
impl<S: ConnectedState> HeosConnection<S> {
    /// Acquire a reference to the [Channel].
    pub async fn channel(&self) -> AsyncMutexGuard<'_, Channel> {
        self.state.channel().lock().await
    }

    /// Send a [RawCommand] over this connection.
    ///
    /// # Errors
    ///
    /// Errors if the connection has an IO error while sending the command and receiving the
    /// response.
    pub async fn raw_command(&self, command: RawCommand) -> Result<RawResponse, std::io::Error> {
        self.state.channel().lock().await.send_raw_command(command).await
    }

    /// Send a [Command] over this connection.
    ///
    /// # Errors
    ///
    /// Errors for any reason found in [CommandError].
    pub async fn command<C>(&self, command: C) -> Result<C::Response, CommandError>
    where
        C: Command,
    {
        self.state.channel().lock().await.send_command(command).await
    }
}

/// Inner state for a [HeosConnection] object that is actively connected to a HEOS endpoint, but
/// does not manage any state.
///
/// AdHoc connections can be used to send commands and receive responses, but do no tracking of the
/// HEOS system's state.
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
    /// Create a connection directly from a [Channel].
    ///
    /// This is an advanced use case, and only useful if you have a custom
    /// [ChannelBackend](channel::ChannelBackend). Usually, you should use e.g.
    /// [`HeosConnection<Created>::connect_any()`].
    pub async fn from_channel(channel: Channel) -> Result<Self, CommandError> {
        let channel = AsyncMutex::new(channel);
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

    /// Subscribe to [change events](data::event) emitted by the HEOS system.
    pub async fn subscribe_event_broadcast(&self) -> BroadcastReceiver<Event> {
        self.state.channel().lock().await.subscribe_event_broadcast()
    }

    /// Initialize a [Stateful] connection.
    ///
    /// This will transition the internal state from [AdHoc] to [Stateful], and the state of the
    /// HEOS system will start being tracked.
    pub async fn init_stateful(self) -> Result<HeosConnection<Stateful>, CommandError> {
        let state = Arc::new(State::init(self.state.channel.into_inner()).await?);
        let event_broadcast = BroadcastSender::new(Channel::EVENT_BROADCAST_BUFFER);
        let event_handle = {
            let state = state.clone();
            let weak_event_broadcast = event_broadcast.downgrade();
            let mut event_recv = state.channel.lock().await.subscribe_event_broadcast();
            tokio::spawn(async move {
                loop {
                    let event = match event_recv.recv().await {
                        Ok(event) => event,
                        Err(_) => break,
                    };

                    match state.handle_event(event.clone()).await {
                        Ok(_) => {},
                        Err(error) => {
                            warn!(?error, "Failed to handle event");
                        }
                    }

                    if let Some(event_broadcast) = weak_event_broadcast.upgrade() {
                        let _ = event_broadcast.send(event);
                    }
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
                event_broadcast,
                event_handle,
            },
        })
    }
}

/// Inner state for a [HeosConnection] object that is actively connected to a HEOS endpoint, and is
/// tracking the overall state of the HEOS system.
///
/// Stateful connections can still be used to directly send commands and receive responses, but it
/// is usually more convenient to use the stateful wrappers around commands that can be found in the
/// [State] object that can be dereferenced from a connection of this type.
#[derive(Debug)]
pub struct Stateful {
    state: Arc<State>,
    event_broadcast: BroadcastSender<Event>,
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

impl HeosConnection<Stateful> {
    /// Subscribe to [change events](data::event) emitted by the HEOS system.
    ///
    /// When subscribing via this method, events will first be fully processed by the stateful
    /// connection before being passed to the user, ensuring that the stateful connection is
    /// up-to-date before user hooks run their logic.
    pub async fn subscribe_event_broadcast(&self) -> BroadcastReceiver<Event> {
        self.state.event_broadcast.subscribe()
    }
}