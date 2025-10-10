use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;

use super::*;

bounded_number_type!(Volume, u16, 0..=100);
impl_try_from_qs!(Volume, "level");
impl_try_from_response_qs!(Volume);

bounded_number_type!(VolumeStep, u16, 0..=10);
impl Default for VolumeStep {
    #[inline]
    fn default() -> Self {
        Self(5)
    }
}

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum MuteState {
    On,
    Off,
}
impl_enum_string_conversions!(MuteState);
impl_try_from_qs!(MuteState, "state");
impl_try_from_response_qs!(MuteState);