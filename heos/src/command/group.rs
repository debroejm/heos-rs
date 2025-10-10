use serde::Serialize;

use crate::command::impl_command;
use crate::data::common::*;
use crate::data::group::*;
use crate::data::player::*;

#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetGroups {}
impl_command!(GetGroups, "group", "get_groups", Vec<GroupInfo>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetGroupInfo {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(GetGroupInfo, "group", "get_group_info", GroupInfo);

// TODO: SetGroup has very specialized responses
#[derive(Serialize, Debug, Clone)]
pub struct SetGroup {
    #[serde(rename = "pid")]
    pub player_ids: Vec<PlayerId>,
}
impl_command!(SetGroup, "group", "set_group", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetGroupVolume {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(GetGroupVolume, "group", "get_volume", Volume);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetGroupVolume {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    pub level: Volume,
}
impl_command!(SetGroupVolume, "group", "set_volume", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GroupVolumeUp {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    pub step: Option<VolumeStep>,
}
impl_command!(GroupVolumeUp, "group", "volume_up", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GroupVolumeDown {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    pub step: Option<VolumeStep>,
}
impl_command!(GroupVolumeDown, "group", "volume_down", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetGroupMute {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(GetGroupMute, "group", "get_mute", MuteState);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetGroupMute {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    pub state: MuteState,
}
impl_command!(SetGroupMute, "group", "set_mute", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct ToggleGroupMute {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
}
impl_command!(ToggleGroupMute, "group", "toggle_mute", ());