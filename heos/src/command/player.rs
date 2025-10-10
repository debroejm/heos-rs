use serde::Serialize;
use std::ops::RangeInclusive;

use crate::command::impl_command;
use crate::data::common::*;
use crate::data::maybe_range;
use crate::data::option::WithOptions;
use crate::data::player::*;
use crate::data::quickselect::*;
use crate::data::song::*;

#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetPlayers {}
impl_command!(GetPlayers, "player", "get_players", Vec<PlayerInfo>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayerInfo {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayerInfo, "player", "get_player_info", PlayerInfo);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayState {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayState, "player", "get_play_state", PlayState);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetPlayState {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub state: PlayState,
}
impl_command!(SetPlayState, "player", "set_play_state", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetNowPlayingMedia {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetNowPlayingMedia, "player", "get_now_playing_media", WithOptions<NowPlayingInfo>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetVolume {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetVolume, "player", "get_volume", Volume);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetVolume {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub level: Volume,
}
impl_command!(SetVolume, "player", "set_volume", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct VolumeUp {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub step: Option<VolumeStep>,
}
impl_command!(VolumeUp, "player", "volume_up", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct VolumeDown {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub step: Option<VolumeStep>,
}
impl_command!(VolumeDown, "player", "volume_down", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetMute {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetMute, "player", "get_mute", MuteState);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetMute {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub state: MuteState,
}
impl_command!(SetMute, "player", "set_mute", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct ToggleMute {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(ToggleMute, "player", "toggle_mute", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayMode {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayMode, "player", "get_play_mode", PlayMode);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetPlayMode {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub repeat: Option<RepeatMode>,
    pub shuffle: Option<ShuffleMode>,
}
impl_command!(SetPlayMode, "player", "set_play_mode", ());

#[derive(Serialize, Debug, Clone)]
pub struct GetQueue {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(serialize_with = "maybe_range::serialize")]
    pub range: Option<RangeInclusive<usize>>,
}
impl_command!(GetQueue, "player", "get_queue", Vec<SongInfo>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayQueueItem {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "qid")]
    pub queue_id: QueueId,
}
impl_command!(PlayQueueItem, "player", "play_queue", ());

#[derive(Serialize, Debug, Clone)]
pub struct RemoveFromQueue {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "qid")]
    pub queue_ids: Vec<QueueId>,
}
impl_command!(RemoveFromQueue, "player", "remove_from_queue", ());

#[derive(Serialize, Debug, Clone)]
pub struct SaveQueue {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub name: String,
}
impl_command!(SaveQueue, "player", "save_queue", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct ClearQueue {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(ClearQueue, "player", "clear_queue", ());

#[derive(Serialize, Debug, Clone)]
pub struct MoveQueue {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "sqid")]
    pub src_queue_ids: Vec<QueueId>,
    #[serde(rename = "dqid")]
    pub dst_queue_id: QueueId,
}
impl_command!(MoveQueue, "player", "move_queue_item", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayNext {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(PlayNext, "player", "play_next", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayPrevious {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(PlayPrevious, "player", "play_previous", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetQuickSelect {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub id: QuickSelectId,
}
impl_command!(SetQuickSelect, "player", "set_quickselect", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayQuickSelect {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub id: QuickSelectId,
}
impl_command!(PlayQuickSelect, "player", "play_quickselect", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetQuickSelects {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub id: Option<QuickSelectId>,
}
impl_command!(GetQuickSelects, "player", "get_quickselects", Vec<QuickSelect>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct CheckUpdate {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(CheckUpdate, "player", "check_update", UpdatePayload);