use serde::{Deserialize, Deserializer};
use std::time::Duration;
use crate::command::CommandError;
use crate::data::common::{MuteState, Volume};
use crate::data::group::GroupId;
use crate::data::player::{PlayState, PlayerId, RepeatMode, ShuffleMode};
use crate::data::response::RawResponse;
use crate::data::system::AccountStatus;

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerStateChanged {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub state: PlayState,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerNowPlayingChanged {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}

fn deserialize_duration<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
    let millis = u64::deserialize(d)?;
    Ok(Duration::from_millis(millis))
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerNowPlayingProgress {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "cur_pos", deserialize_with = "deserialize_duration")]
    pub elapsed: Duration,
    #[serde(deserialize_with = "deserialize_duration")]
    pub duration: Duration,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlayerPlaybackError {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub error: String,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerQueueChanged {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerVolumeChanged {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub level: Volume,
    pub mute: MuteState,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerRepeatModeChanged {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub repeat: RepeatMode,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PlayerShuffleModeChanged {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub shuffle: ShuffleMode,
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct GroupVolumeChanged {
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    pub level: Volume,
    pub mute: MuteState,
}

#[derive(Debug, Clone)]
pub struct UserChanged {
    pub account: AccountStatus,
}

#[derive(Debug, Clone)]
pub enum Event {
    SourcesChanged,
    PlayersChanged,
    GroupsChanged,
    PlayerStateChanged(PlayerStateChanged),
    PlayerNowPlayingChanged(PlayerNowPlayingChanged),
    PlayerNowPlayingProgress(PlayerNowPlayingProgress),
    PlayerPlaybackError(PlayerPlaybackError),
    PlayerQueueChanged(PlayerQueueChanged),
    PlayerVolumeChanged(PlayerVolumeChanged),
    PlayerRepeatModeChanged(PlayerRepeatModeChanged),
    PlayerShuffleModeChanged(PlayerShuffleModeChanged),
    GroupVolumeChanged(GroupVolumeChanged),
    UserChanged(UserChanged),
}

#[derive(thiserror::Error, Debug)]
pub enum ParseEventError {
    #[error("failed to parse Event from RawResponse.message: {0}")]
    DeserializeError(#[from] serde_qs::Error),
    #[error(transparent)]
    CommandError(#[from] CommandError),
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