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
#[derive(Serialize, Debug, Clone, Copy)]
pub struct RegisterForChangeEvents {
    pub enable: ChangeEventsEnabled,
}
impl_command!(RegisterForChangeEvents, "system", "register_for_change_events", ());

/// Retrieves the current signed-in HEOS username.
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct CheckAccount {}
impl_command!(CheckAccount, "system", "check_account", AccountStatus);

/// Sign in to a HEOS account using the specified username and password.
///
/// <div class="warning">
/// The TCP connection to local HEOS devices is NOT encrypted, meaning passwords are sent via
/// plain-text! It may be better to enforce signing in through the official HEOS app instead.
/// </div>
#[derive(Serialize, Debug, Clone)]
pub struct SignIn {
    pub username: String,
    pub password: String,
}
impl_command!(SignIn, "system", "sign_in", ());

/// Sign out the current HEOS account.
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct SignOut {}
impl_command!(SignOut, "system", "sign_out", ());

/// Retrieve a heartbeat response.
///
/// This is effectively a ping.
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct HeartBeat {}
impl_command!(HeartBeat, "system", "heart_beat", ());

/// Reboot the connected HEOS device.
///
/// Note that this command can only be used to reboot the HEOS device that is currently connected.
/// To reboot other HEOS devices on the network, separate connections will need to be made to them.
#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct Reboot {}
impl_command!(Reboot, "system", "reboot", ());