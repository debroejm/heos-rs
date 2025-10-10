use qstring::QString;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;

use crate::command::CommandError;
use crate::data::{impl_enum_string_conversions, impl_try_from_response_qs};

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum ChangeEventsEnabled {
    On,
    Off,
}
impl_enum_string_conversions!(ChangeEventsEnabled);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountStatus {
    SignedOut,
    SignedIn(String),
}
impl_try_from_response_qs!(AccountStatus);

impl<'a> TryFrom<&'a QString> for AccountStatus {
    type Error = CommandError;

    fn try_from(qs: &'a QString) -> Result<Self, Self::Error> {
        if qs.get("signed_out").is_some() {
            Ok(Self::SignedOut)
        } else if qs.get("signed_in").is_some() {
            let username = qs.get("un")
                .ok_or(CommandError::response_missing_field("message.un"))?;
            Ok(Self::SignedIn(username.to_string()))
        } else {
            Err(CommandError::response_missing_field("message.signed_out/signed_in"))
        }
    }
}

impl TryFrom<QString> for AccountStatus {
    type Error = CommandError;

    #[inline]
    fn try_from(qs: QString) -> Result<Self, Self::Error> {
        Self::try_from(&qs)
    }
}