//! Commands to query/control individual players.
//!
//! See [crate::data::player] for more information on players.

use serde::Serialize;
use std::ops::RangeInclusive;

use crate::command::impl_command;
use crate::data::common::*;
use crate::data::maybe_range;
use crate::data::option::WithOptions;
use crate::data::player::*;
use crate::data::quickselect::*;
use crate::data::song::*;

/// Retrieve all known players.
#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetPlayers {}
impl_command!(GetPlayers, "player", "get_players", Vec<PlayerInfo>);

/// Retrieve a specific player by [ID](PlayerId).
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayerInfo {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayerInfo, "player", "get_player_info", PlayerInfo);

/// Retrieve a player's [PlayState].
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayState {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayState, "player", "get_play_state", PlayState);

/// Set a player's [PlayState].
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetPlayState {
    /// ID of the player to set.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New PlayState to set.
    pub state: PlayState,
}
impl_command!(SetPlayState, "player", "set_play_state", ());

/// Retrieve a player's currently playing media information.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetNowPlayingMedia {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetNowPlayingMedia, "player", "get_now_playing_media", WithOptions<NowPlayingInfo>);

/// Retrieve a player's current volume level.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetVolume {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetVolume, "player", "get_volume", Volume);

/// Set a player's volume level.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetVolume {
    /// ID of the player to set.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New volume level to set.
    pub level: Volume,
}
impl_command!(SetVolume, "player", "set_volume", ());

/// Increment a player's volume level.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct VolumeUp {
    /// ID of the player to increment.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// How much to increment by.
    ///
    /// This parameter may be omitted (`None`), and defaults to an increment step of `5`.
    pub step: Option<VolumeStep>,
}
impl_command!(VolumeUp, "player", "volume_up", ());

/// Decrement a player's volume level.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct VolumeDown {
    /// ID of the player to decrement.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// How much to decrement by.
    ///
    /// This parameter may be omitted (`None`), and defaults to a decrement step of `5`.
    pub step: Option<VolumeStep>,
}
impl_command!(VolumeDown, "player", "volume_down", ());

/// Retrieve a player's [MuteState].
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetMute {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetMute, "player", "get_mute", MuteState);

/// Set a player's [MuteState].
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetMute {
    /// ID of the player to set.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New MuteState to set.
    pub state: MuteState,
}
impl_command!(SetMute, "player", "set_mute", ());

/// Toggle a player's [MuteState].
///
/// If it was previously off, it will be toggled to be on, and vice versa.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct ToggleMute {
    /// ID of the player to toggle.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(ToggleMute, "player", "toggle_mute", ());

/// Retrieve a player's [PlayMode].
///
/// This retrieves both [RepeatMode] and [ShuffleMode].
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayMode {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayMode, "player", "get_play_mode", PlayMode);

/// Set a player's [PlayMode].
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetPlayMode {
    /// ID of the player to set.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New repeat mode to set.
    ///
    /// This may be omitted (`None`) to set only [ShuffleMode].
    pub repeat: Option<RepeatMode>,
    /// New shuffle mode to set.
    ///
    /// This may be omitted (`None`) to set only [RepeatMode].
    pub shuffle: Option<ShuffleMode>,
}
impl_command!(SetPlayMode, "player", "set_play_mode", ());

/// Retrieve a player's current queue of songs.
#[derive(Serialize, Debug, Clone)]
pub struct GetQueue {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// Range of songs to retrieve.
    ///
    /// Zero-indexed.
    ///
    /// This may be omitted (`None`), and defaults to all songs up to a maximum of 100 records per
    /// response.
    // TODO: Figure out how HEOS paginated responses work
    #[serde(serialize_with = "maybe_range::serialize")]
    pub range: Option<RangeInclusive<usize>>,
}
impl_command!(GetQueue, "player", "get_queue", Vec<SongInfo>);

/// Play a particular song from a player's queue.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayQueueItem {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of the queued song to play.
    #[serde(rename = "qid")]
    pub queue_id: QueueId,
}
impl_command!(PlayQueueItem, "player", "play_queue", ());

/// Remove song(s) from a player's queue.
#[derive(Serialize, Debug, Clone)]
pub struct RemoveFromQueue {
    /// ID of the player to remove a song from.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// One or more IDs of queued songs to remove.
    #[serde(rename = "qid")]
    pub queue_ids: Vec<QueueId>,
}
impl_command!(RemoveFromQueue, "player", "remove_from_queue", ());

/// Save the current queue as a new HEOS playlist.
#[derive(Serialize, Debug, Clone)]
pub struct SaveQueue {
    /// ID of the player to retrieve the queue from.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// Name of the new playlist.
    pub name: String,
}
impl_command!(SaveQueue, "player", "save_queue", ());

/// Clear a player's queue of songs.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct ClearQueue {
    /// ID of the player to clear.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(ClearQueue, "player", "clear_queue", ());

/// Move songs around in a queue.
#[derive(Serialize, Debug, Clone)]
pub struct MoveQueue {
    /// ID of the player to move queued songs on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// One or more IDs of queued songs to move.
    #[serde(rename = "sqid")]
    pub src_queue_ids: Vec<QueueId>,
    /// ID of the destination queued song to move the src songs to.
    ///
    /// The songs will be inserted right before the destination.
    #[serde(rename = "dqid")]
    pub dst_queue_id: QueueId,
}
impl_command!(MoveQueue, "player", "move_queue_item", ());

/// Play the next song.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayNext {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(PlayNext, "player", "play_next", ());

/// Play the previous song.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayPrevious {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(PlayPrevious, "player", "play_previous", ());

/// Set a QuickSelect slot to use the currently playing source.
///
/// Note: This is not supported by all HEOS devices.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct SetQuickSelect {
    /// ID of the player to set.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of the QuickSelect slot.
    pub id: QuickSelectId,
}
impl_command!(SetQuickSelect, "player", "set_quickselect", ());

/// Play a QuickSelect slot.
///
/// Note: This is not supported by all HEOS devices.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayQuickSelect {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of the QuickSelect slot.
    pub id: QuickSelectId,
}
impl_command!(PlayQuickSelect, "player", "play_quickselect", ());

/// Retrieve information on QuickSelect slot(s).
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetQuickSelects {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of QuickSelect slot to retrieve.
    ///
    /// This may be omitted (`None`), and will retrieve all QuickSelect slots by default.
    pub id: Option<QuickSelectId>,
}
impl_command!(GetQuickSelects, "player", "get_quickselects", Vec<QuickSelect>);

/// Retrieve whether an update exists for a player.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct CheckUpdate {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(CheckUpdate, "player", "check_update", UpdatePayload);