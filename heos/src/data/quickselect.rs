use serde::Deserialize;

use super::*;

id_type!(QuickSelectId);

#[derive(Deserialize, Debug, Clone)]
pub struct QuickSelect {
    pub id: QuickSelectId,
    pub name: String,
}
impl_try_from_response_payload!(QuickSelect);
impl_try_from_response_payload!(Vec<QuickSelect>);