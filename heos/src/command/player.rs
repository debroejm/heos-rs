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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetPlayers;
/// use heos::data::player::PlayerInfo;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let players: Vec<PlayerInfo> = heos.command(GetPlayers::default()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetPlayers {}
impl_command!(GetPlayers, "player", "get_players", Vec<PlayerInfo>);

/// Retrieve a specific player by [ID](PlayerId).
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetPlayerInfo;
/// use heos::data::player::{PlayerId, PlayerInfo};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let player: PlayerInfo = heos.command(GetPlayerInfo {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayerInfo {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayerInfo, "player", "get_player_info", PlayerInfo);

/// Retrieve a player's [PlayState].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetPlayState;
/// use heos::data::player::{PlayerId, PlayState};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let play_state: PlayState = heos.command(GetPlayState {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayState {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayState, "player", "get_play_state", PlayState);

/// Set a player's [PlayState].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::SetPlayState;
/// use heos::data::player::{PlayerId, PlayState};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetPlayState {
///     player_id: PlayerId::from(42),
///     state: PlayState::Play,
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetNowPlayingMedia;
/// use heos::data::option::WithOptions;
/// use heos::data::player::PlayerId;
/// use heos::data::song::NowPlayingInfo;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let now_playing: WithOptions<NowPlayingInfo> = heos.command(GetNowPlayingMedia {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetNowPlayingMedia {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetNowPlayingMedia, "player", "get_now_playing_media", WithOptions<NowPlayingInfo>);

/// Retrieve a player's current volume level.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetVolume;
/// use heos::data::common::Volume;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let volume: Volume = heos.command(GetVolume {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetVolume {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetVolume, "player", "get_volume", Volume);

/// Set a player's volume level.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::SetVolume;
/// use heos::data::common::Volume;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetVolume {
///     player_id: PlayerId::from(42),
///     level: Volume::try_from(50).unwrap(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::VolumeUp;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(VolumeUp {
///     player_id: PlayerId::from(42),
///     step: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::VolumeDown;
/// use heos::data::common::VolumeStep;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(VolumeDown {
///     player_id: PlayerId::from(42),
///     step: Some(VolumeStep::try_from(3).unwrap()),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetMute;
/// use heos::data::common::MuteState;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let mute_state: MuteState = heos.command(GetMute {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetMute {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetMute, "player", "get_mute", MuteState);

/// Set a player's [MuteState].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::SetMute;
/// use heos::data::common::MuteState;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetMute {
///     player_id: PlayerId::from(42),
///     state: MuteState::On,
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::ToggleMute;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(ToggleMute {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetPlayMode;
/// use heos::data::player::{PlayerId, PlayMode, RepeatMode, ShuffleMode};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let play_mode: PlayMode = heos.command(GetPlayMode {
///     player_id: PlayerId::from(42),
/// }).await?;
/// let repeat_mode = play_mode.repeat;
/// let shuffle_mode = play_mode.shuffle;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetPlayMode {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(GetPlayMode, "player", "get_play_mode", PlayMode);

/// Set a player's [PlayMode].
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::SetPlayMode;
/// use heos::data::player::{PlayerId, PlayMode, RepeatMode, ShuffleMode};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetPlayMode {
///     player_id: PlayerId::from(42),
///     repeat: Some(RepeatMode::All),
///     shuffle: Some(ShuffleMode::Off),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetQueue;
/// use heos::data::player::PlayerId;
/// use heos::data::song::SongInfo;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let queue: Vec<SongInfo> = heos.command(GetQueue {
///     player_id: PlayerId::from(42),
///     range: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::PlayQueueItem;
/// use heos::data::player::PlayerId;
/// use heos::data::song::QueueId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(PlayQueueItem {
///     player_id: PlayerId::from(42),
///     queue_id: QueueId::from(2),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::RemoveFromQueue;
/// use heos::data::player::PlayerId;
/// use heos::data::song::QueueId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(RemoveFromQueue {
///     player_id: PlayerId::from(42),
///     queue_ids: vec![QueueId::from(1), QueueId::from(2)],
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::SaveQueue;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SaveQueue {
///     player_id: PlayerId::from(42),
///     name: "MyNewPlaylist".to_string(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::ClearQueue;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(ClearQueue {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct ClearQueue {
    /// ID of the player to clear.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(ClearQueue, "player", "clear_queue", ());

/// Move songs around in a queue.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::MoveQueue;
/// use heos::data::player::PlayerId;
/// use heos::data::song::QueueId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(MoveQueue {
///     player_id: PlayerId::from(42),
///     src_queue_ids: vec![QueueId::from(3), QueueId::from(4)],
///     dst_queue_id: QueueId::from(1),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::PlayNext;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(PlayNext {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayNext {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(PlayNext, "player", "play_next", ());

/// Play the previous song.
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::PlayPrevious;
/// use heos::data::player::PlayerId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(PlayPrevious {
///     player_id: PlayerId::from(42),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::SetQuickSelect;
/// use heos::data::player::PlayerId;
/// use heos::data::quickselect::QuickSelectId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(SetQuickSelect {
///     player_id: PlayerId::from(42),
///     id: QuickSelectId::try_from(3).unwrap(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::PlayQuickSelect;
/// use heos::data::player::PlayerId;
/// use heos::data::quickselect::QuickSelectId;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// heos.command(PlayQuickSelect {
///     player_id: PlayerId::from(42),
///     id: QuickSelectId::try_from(3).unwrap(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::GetQuickSelects;
/// use heos::data::player::PlayerId;
/// use heos::data::quickselect::QuickSelect;
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let quick_selects: Vec<QuickSelect> = heos.command(GetQuickSelects {
///     player_id: PlayerId::from(42),
///     id: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
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
///
/// ```
/// # use heos::ConnectError;
/// use heos::HeosConnection;
/// use heos::command::player::CheckUpdate;
/// use heos::data::player::{PlayerId, UpdateAvailable};
/// use std::time::Duration;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() -> Result<(), ConnectError> {
/// # heos::install_doctest_handler();
/// let heos = HeosConnection::connect_any(Duration::from_secs(1)).await?;
/// let update: UpdateAvailable = heos.command(CheckUpdate {
///     player_id: PlayerId::from(42),
/// }).await?.update;
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Debug, Clone, Copy)]
pub struct CheckUpdate {
    /// ID of the player to retrieve.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}
impl_command!(CheckUpdate, "player", "check_update", UpdatePayload);