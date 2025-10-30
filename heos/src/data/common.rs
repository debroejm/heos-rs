//! Data types that are common to more than one module.

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;

use super::*;

bounded_number_type! {
    /// Volume level of [players](player) and [groups](group).
    ///
    /// Valid range is `0..=100`, where `0` is considered "muted".
    pub struct Volume(u16, 0..=100);
}
impl_try_from_qs!(Volume, "level");
impl_try_from_response_qs!(Volume);

bounded_number_type! {
    /// Incremental volume "step" to increment or decrement [Volume] by.
    ///
    /// Valid range is `0..=10`, and defaults to `5`.
    pub struct VolumeStep(u16, 0..=10);
}
impl Default for VolumeStep {
    #[inline]
    fn default() -> Self {
        Self(5)
    }
}

/// The state of a [player] or [group] being "muted".
#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum MuteState {
    /// The [player] or [group] is muted.
    On,
    /// The [player] or [group] is NOT muted.
    Off,
}
impl_enum_string_conversions!(MuteState);
impl_try_from_qs!(MuteState, "state");
impl_try_from_response_qs!(MuteState);