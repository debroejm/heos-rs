//! Raw response data types.

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

/// Raw response HEOS metadata.
#[derive(Deserialize, Debug)]
pub struct RawResponseHeos {
    /// The full command that produced this response.
    ///
    /// For commands, this will be of the format "<group>/<command>".
    ///
    /// For events, this will be of the format "event/<event>".
    pub command: String,
    /// Whether this command was successful (`true`) or not (`false`).
    #[serde(default, deserialize_with = "deserialize_result")]
    pub result: Option<bool>,
    /// The "message" part of the response.
    ///
    /// This value is a query-string that contains all the parameters sent to the HEOS system as
    /// part of the original command. It also sometime contains _additional_ values that are yielded
    /// as part of the response.
    #[serde(default)]
    pub message: String,
}

/// The data for a raw response received from the HEOS system.
///
/// Note that this raw response can also represent [change events](super::event). Many data types
/// can be parsed from a raw response using [TryFrom].
#[derive(Deserialize, Debug)]
pub struct RawResponse {
    /// HEOS metadata of the response.
    pub heos: RawResponseHeos,
    /// Optional raw JSON payload.
    ///
    /// For commands that yield large amounts of data, that data is usually encoded in the JSON
    /// payload. The top-level JSON value can be either a list or a map.
    pub payload: Option<Value>,
    /// Optional "option" JSON.
    ///
    /// For commands that can retrieve service option values, they will be yielded in this JSON.
    pub options: Option<Value>,
}

impl RawResponse {
    /// Validate that a response represents a successful command execution.
    ///
    /// # Errors
    ///
    /// If the command did not successfully execute, this will parse out a [CommandError] from the
    /// raw response.
    pub fn validate_command(&self) -> Result<(), CommandError> {
        let result = self.heos.result
            .ok_or(CommandError::response_missing_field("heos.result"))?;

        if !result {
            Err(CommandError::from_message(&self.heos.message))
        } else {
            Ok(())
        }
    }

    /// Try to parse the message SEQUENCE from the response.
    ///
    /// Commands can specify a SEQUENCE parameter that is duplicated in the response, in order to
    /// easily associate responses to their original commands. This attempts to parse out that
    /// SEQUENCE value, if it exists in the response.
    ///
    /// # Errors
    ///
    /// Errors if the SEQUENCE parameter does not exist in the response.
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