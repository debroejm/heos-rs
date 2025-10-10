use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::command::CommandError;

fn deserialize_result<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
    let result_str = Option::<String>::deserialize(d)?;
    match result_str {
        Some(result_str) => {
            match result_str.as_str() {
                "success" => Ok(Some(true)),
                "fail" => Ok(Some(false)),
                other => Err(serde::de::Error::custom(format!("unknown result str: '{other}'"))),
            }
        },
        None => Ok(None),
    }
}

#[derive(Deserialize, Debug)]
pub struct RawResponseHeos {
    pub command: String,
    #[serde(default, deserialize_with = "deserialize_result")]
    pub result: Option<bool>,
    #[serde(default)]
    pub message: String,
}

#[derive(Deserialize, Debug)]
pub struct RawResponse {
    pub heos: RawResponseHeos,
    pub payload: Option<Value>,
    pub options: Option<Value>,
}

impl RawResponse {
    pub fn validate_command(&self) -> Result<(), CommandError> {
        let result = self.heos.result
            .ok_or(CommandError::response_missing_field("heos.result"))?;

        if !result {
            Err(CommandError::from_message(&self.heos.message))
        } else {
            Ok(())
        }
    }

    pub fn try_msg_id(&self) -> Result<Option<u64>, CommandError> {
        let qs = qstring::QString::from(self.heos.message.as_str());
        let val = match qs.get("SEQUENCE") {
            Some(val) => val,
            None => return Ok(None),
        };
        let msg_id = val.parse().map_err(|error| {
            CommandError::MalformedResponse(format!("could not parse 'SEQUENCE': {error}"))
        })?;
        Ok(Some(msg_id))
    }
}

impl TryFrom<RawResponse> for () {
    type Error = CommandError;

    #[inline]
    fn try_from(_: RawResponse) -> Result<Self, Self::Error> {
        Ok(())
    }
}