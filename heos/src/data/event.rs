/// Data types for change events.
///
/// The HEOS system sends change events whenever the internal HEOS state changes in some fashion.
/// This can be e.g. when the currently playing song changes to a new song, when a player's volume
/// or play mode changes, or even when new players are added to the system, among others. See
/// [Event] for a full list of possible events.
///
/// By default, the HEOS system does not send change events until a connection has registered for
/// them via the [RegisterForChangeEvents](crate::command::system::RegisterForChangeEvents) command.

use serde::{Deserialize, Deserializer};
use std::time::Duration;

use crate::command::CommandError;
use crate::data::common::{MuteState, Volume};
use crate::data::group::GroupId;
use crate::data::player::{PlayState, PlayerId, RepeatMode, ShuffleMode};
use crate::data::response::RawResponse;
use crate::data::system::AccountStatus;

/// Additional data for when a player's [PlayState] changes.
// TODO: fix name to be PlayerPlayStateChanged
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerStateChanged {
    /// ID of the player that changed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// The new play state.
    pub state: PlayState,
}

/// Additional data for when a player's currently playing track changes.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerNowPlayingChanged {
    /// ID of the player that changed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}

fn deserialize_duration<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let millis = u64::deserialize(d)?;
    Ok(Duration::from_millis(millis))
}

/// Additional data for when a player's currently playing track progresses.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerNowPlayingProgress {
    /// ID of the player that progressed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// How much of the currently playing track has elapsed.
    #[serde(rename = "cur_pos", deserialize_with = "deserialize_duration")]
    pub elapsed: Duration,
    /// The total duration of the currently playing track.
    #[serde(deserialize_with = "deserialize_duration")]
    pub duration: Duration,
}

/// Additional data for when a player has a playback error.
#[derive(Deserialize, Debug, Clone)]
pub struct PlayerPlaybackError {
    /// ID of the player that errored.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// Error message.
    pub error: String,
}

/// Additional data for when a player's queue changes.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerQueueChanged {
    /// ID of the player that changed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}

/// Additional data for when a player's volume changes.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerVolumeChanged {
    /// ID of the player that changed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New volume level.
    pub level: Volume,
    /// New mute state.
    pub mute: MuteState,
}

/// Additional data for when a player's repeat mode changes.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerRepeatModeChanged {
    /// ID of the player that changed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New repeat mode.
    pub repeat: RepeatMode,
}

/// Additional data for when a player's shuffle mode changes.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerShuffleModeChanged {
    /// ID of the player that changed.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// New shuffle mode.
    pub shuffle: ShuffleMode,
}

/// Additional data for when a group's volume changes.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct GroupVolumeChanged {
    /// ID of the group that changed.
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    /// New volume level.
    pub level: Volume,
    /// New mute state.
    pub mute: MuteState,
}

/// Additional data for when the HEOS account status changes.
#[derive(Debug, Clone)]
pub struct UserChanged {
    /// New account status.
    pub account: AccountStatus,
}

/// Enumeration of all possible events that the HEOS system can send.
#[derive(Debug, Clone)]
pub enum Event {
    /// The available music sources changed.
    ///
    /// Any cached values should be invalidated and re-queried.
    SourcesChanged,

    /// The connected players changed.
    ///
    /// Any cached values should be invalidated and re-queried.
    PlayersChanged,

    /// The group configurations changed.
    ///
    /// Any cached values should be invalidated and re-queried.
    GroupsChanged,

    /// A player's play state changed.
    ///
    /// The event contains the new play state.
    PlayerStateChanged(PlayerStateChanged),

    /// A player's now playing track changed.
    ///
    /// Any cached values should be invalidated and re-queried.
    PlayerNowPlayingChanged(PlayerNowPlayingChanged),

    /// A player's now playing track progressed.
    ///
    /// The event contains the new progression state.
    PlayerNowPlayingProgress(PlayerNowPlayingProgress),

    /// A player had a playback error.
    ///
    /// The event contains the error details.
    PlayerPlaybackError(PlayerPlaybackError),

    /// A player's queue changed.
    ///
    /// Any cached values should be invalidated and re-queried.
    PlayerQueueChanged(PlayerQueueChanged),

    /// A player's volume changed.
    ///
    /// The event contains the new volume level and mute state.
    PlayerVolumeChanged(PlayerVolumeChanged),

    /// A player's repeat mode changed.
    ///
    /// The event contains the new repeat mode.
    PlayerRepeatModeChanged(PlayerRepeatModeChanged),

    /// A player's shuffle mode changed.
    ///
    /// The event contains the new shuffle mode.
    PlayerShuffleModeChanged(PlayerShuffleModeChanged),

    /// A group's volume changed.
    ///
    /// The event contains the new volume level and mute state.
    GroupVolumeChanged(GroupVolumeChanged),

    /// The signed-in HEOS user changed.
    ///
    /// The event contains the new account status.
    UserChanged(UserChanged),
}

/// Errors that can occur when parsing an event from a raw response.
#[derive(thiserror::Error, Debug)]
pub enum ParseEventError {
    /// The event could not be successfully parsed from the raw response message field.
    #[error("failed to parse Event from RawResponse.message: {0}")]
    DeserializeError(#[from] serde_qs::Error),
    /// Some other [CommandError] occurred.
    #[error(transparent)]
    CommandError(#[from] CommandError),
    /// The event type specified in the raw response is unknown.
    #[error("unknown event type: {0}")]
    UnknownEvent(String),
}

impl TryFrom<RawResponse> for Event {
    type Error = ParseEventError;
    fn try_from(response: RawResponse) -> Result<Self, Self::Error> {
        match response.heos.command.as_str() {
            "event/sources_changed" => Ok(Event::SourcesChanged),
            "event/players_changed" => Ok(Event::PlayersChanged),
            "event/groups_changed" => Ok(Event::GroupsChanged),
            "event/player_state_changed" => Ok(Event::PlayerStateChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/player_now_playing_changed" => Ok(Event::PlayerNowPlayingChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/player_now_playing_progress" => Ok(Event::PlayerNowPlayingProgress(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/player_playback_error" => Ok(Event::PlayerPlaybackError(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/player_queue_changed" => Ok(Event::PlayerQueueChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/player_volume_changed" => Ok(Event::PlayerVolumeChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/repeat_mode_changed" => Ok(Event::PlayerRepeatModeChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/shuffle_mode_changed" => Ok(Event::PlayerShuffleModeChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/group_volume_changed" => Ok(Event::GroupVolumeChanged(
                serde_qs::from_str(response.heos.message.as_str())?
            )),
            "event/user_changed" => {
                let qs = qstring::QString::from(response.heos.message.as_str());
                let account = AccountStatus::try_from(qs)?;
                Ok(Event::UserChanged(UserChanged {
                    account,
                }))
            },
            unknown => Err(ParseEventError::UnknownEvent(unknown.to_string())),
        }
    }
}