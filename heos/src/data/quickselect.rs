//! Data types representing QuickSelect slots.
//!
//! QuickSelect is limited to certain HEOS devices, and allows easily playing a saved preset.

use serde::{Deserialize, Serialize};

use super::*;

bounded_number_type! {
    /// ID representing a QuickSelect slot.
    ///
    /// This is limited to the range `1..=6`.
    pub struct QuickSelectId(i64, 1..=6);
}

/// Information about a specific QuickSelect slot.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct QuickSelect {
    /// ID of the QuickSelect slot.
    pub id: QuickSelectId,
    /// User-defined name of the QuickSelect slot.
    pub name: String,
}
impl_try_from_response_payload!(QuickSelect);
impl_try_from_response_payload!(Vec<QuickSelect>);