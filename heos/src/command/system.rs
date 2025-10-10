use serde::Serialize;

use crate::command::impl_command;
use crate::data::system::*;

#[derive(Serialize, Debug, Clone, Copy)]
pub struct RegisterForChangeEvents {
    pub enable: ChangeEventsEnabled,
}
impl_command!(RegisterForChangeEvents, "system", "register_for_change_events", ());

#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct CheckAccount {}
impl_command!(CheckAccount, "system", "check_account", AccountStatus);

#[derive(Serialize, Debug, Clone)]
pub struct SignIn {
    pub username: String,
    pub password: String,
}
impl_command!(SignIn, "system", "sign_in", ());

#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct SignOut {}
impl_command!(SignOut, "system", "sign_out", ());

#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct HeartBeat {}
impl_command!(HeartBeat, "system", "heart_beat", ());

#[derive(Serialize, Debug, Clone, Copy, Default)]
pub struct Reboot {}
impl_command!(Reboot, "system", "reboot", ());