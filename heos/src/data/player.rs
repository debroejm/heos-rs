use educe::Educe;
use qstring::QString;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::str::FromStr;
use strum::EnumString;

use super::*;
use crate::command::CommandError;
use crate::data::group::GroupId;

id_type!(PlayerId);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum NetworkType {
    Wired,
    WiFi,
    Unknown,
}
impl_enum_string_conversions!(NetworkType);

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(try_from = "i64")]
pub enum LineOutType {
    None,
    Variable,
    Fixed,
}

impl TryFrom<i64> for LineOutType {
    type Error = String;

    #[inline]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(LineOutType::None),
            1 => Ok(LineOutType::Variable),
            2 => Ok(LineOutType::Fixed),
            other => Err(format!("Unknown line_out type: '{other}'")),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(try_from = "i64")]
pub enum LineOutControlType {
    None,
    IR,
    Trigger,
    Network,
}

impl TryFrom<i64> for LineOutControlType {
    type Error = String;

    #[inline]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        match value {
            0 | 1 => Ok(LineOutControlType::None),
            2 => Ok(LineOutControlType::IR),
            3 => Ok(LineOutControlType::Trigger),
            4 => Ok(LineOutControlType::Network),
            other => Err(format!("Unknown line_out_control type: '{other}'")),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PlayerInfo {
    pub name: String,
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "gid")]
    pub group_id: Option<GroupId>,
    pub model: String,
    pub version: String,
    pub ip: IpAddr,
    pub network: NetworkType,
    #[serde(rename = "lineout")]
    pub line_out: LineOutType,
    #[serde(rename = "control")]
    pub line_out_control: Option<LineOutControlType>,
    pub serial: Option<String>,
}
impl_try_from_response_payload!(PlayerInfo);
impl_try_from_response_payload!(Vec<PlayerInfo>);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum PlayState {
    Play,
    Pause,
    Stop,
}
impl_enum_string_conversions!(PlayState);
impl_try_from_qs!(PlayState, "state");
impl_try_from_response_qs!(PlayState);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum RepeatMode {
    #[strum(serialize = "on_all")]
    All,
    #[strum(serialize = "on_one")]
    One,
    Off,
}
impl_enum_string_conversions!(RepeatMode);
impl_try_from_qs!(RepeatMode, "repeat");
impl_try_from_response_qs!(RepeatMode);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum ShuffleMode {
    On,
    Off,
}
impl_enum_string_conversions!(ShuffleMode);
impl_try_from_qs!(ShuffleMode, "shuffle");
impl_try_from_response_qs!(ShuffleMode);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayMode {
    pub repeat: RepeatMode,
    pub shuffle: ShuffleMode,
}
impl_try_from_response_qs!(PlayMode);

impl TryFrom<QString> for PlayMode {
    type Error = CommandError;

    #[inline]
    fn try_from(qs: QString) -> Result<Self, Self::Error> {
        let repeat = RepeatMode::try_from(&qs)?;
        let shuffle = ShuffleMode::try_from(qs)?;
        Ok(Self {
            repeat,
            shuffle,
        })
    }
}

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
pub enum UpdateAvailable {
    #[strum(serialize = "update_none")]
    None,
    #[strum(serialize = "update_exist")]
    Exists,
}
impl_enum_string_conversions!(UpdateAvailable);

#[derive(Serialize, Deserialize, Educe, Debug, Clone, Copy)]
#[educe(Into(UpdateAvailable))]
pub struct UpdatePayload {
    pub update: UpdateAvailable,
}
impl_try_from_response_payload!(UpdatePayload);

#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "i64")]
pub enum AddToQueueType {
    PlayNow,
    PlayNext,
    AddToEnd,
    ReplaceAndPlay,
}

impl From<AddToQueueType> for i64 {
    #[inline]
    fn from(value: AddToQueueType) -> Self {
        match value {
            AddToQueueType::PlayNow => 1,
            AddToQueueType::PlayNext => 2,
            AddToQueueType::AddToEnd => 3,
            AddToQueueType::ReplaceAndPlay => 4,
        }
    }
}