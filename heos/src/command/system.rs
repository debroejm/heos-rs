//! Commands that affect the HEOS system.
//! 
//! These commands include e.g. HEOS account management.

use serde::Serialize;

use crate::command::impl_command;
use crate::data::system::*;

/// Set whether change events should be received.
///
/// For [AdHoc](crate::AdHoc) connections, by default this is off. This command can be manually sent
/// to start receiving events, which can be received via
/// [`HeosConnection<AdHoc>::subscribe_event_broadcast()`](crate::HeosConnection<crate::AdHoc>::subscribe_event_broadcast).
///
/// For [Stateful](crate::Stateful) connections, this is automatically turned on, and should not be
/// turned off, as it is required for the internal state to be kept up-to-date.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::system::RegisterForChangeEvents;
/// use heos::data::system::ChangeEventsEnabled;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(RegisterForChangeEvents {
///     enable: ChangeEventsEnabled::On,
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct RegisterForChangeEvents {
    pub enable: ChangeEventsEnabled,
}
impl_command!(RegisterForChangeEvents, "system", "register_for_change_events", ());

/// Retrieves the current signed-in HEOS username.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::system::CheckAccount;
/// use heos::data::system::AccountStatus;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let status: AccountStatus = heos.command(CheckAccount::default()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct CheckAccount {}
impl_command!(CheckAccount, "system", "check_account", AccountStatus);

/// Sign in to a HEOS account using the specified username and password.
///
/// <div class="warning">
/// The TCP connection to local HEOS devices is NOT encrypted, meaning passwords are sent via
/// plain-text! It may be better to enforce signing in through the official HEOS app instead.
/// </div>
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::system::SignIn;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SignIn {
///     username: "my_username".to_string(),
///     password: "my_password".to_string(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone)]
pub struct SignIn {
    pub username: String,
    pub password: String,
}
impl_command!(SignIn, "system", "sign_in", ());

/// Sign out the current HEOS account.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::system::SignOut;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SignOut::default()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct SignOut {}
impl_command!(SignOut, "system", "sign_out", ());

/// Retrieve a heartbeat response.
///
/// This is effectively a ping.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::system::HeartBeat;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(HeartBeat::default()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct HeartBeat {}
impl_command!(HeartBeat, "system", "heart_beat", ());

/// Reboot the connected HEOS device.
///
/// Note that this command can only be used to reboot the HEOS device that is currently connected.
/// To reboot other HEOS devices on the network, separate connections will need to be made to them.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::system::Reboot;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(Reboot::default()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct Reboot {}
impl_command!(Reboot, "system", "reboot", ());