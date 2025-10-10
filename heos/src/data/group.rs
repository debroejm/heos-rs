use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;

use super::*;
use crate::data::player::PlayerId;

id_type!(GroupId);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum GroupRole {
    Leader,
    Member,
}
impl_enum_string_conversions!(GroupRole);

#[derive(Deserialize, Debug)]
pub struct GroupPlayer {
    pub name: String,
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub role: GroupRole,
}

#[derive(Deserialize, Debug)]
pub struct GroupInfo {
    pub name: String,
    #[serde(rename = "gid")]
    pub group_id: GroupId,
    pub players: Vec<GroupPlayer>,
}
impl_try_from_response_payload!(GroupInfo);
impl_try_from_response_payload!(Vec<GroupInfo>);