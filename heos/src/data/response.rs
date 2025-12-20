//! Raw response data types.

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::VecDeque;

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

enum RecursiveJsonStringIterInner<'a> {
    Exhausted,
    //String(&'a mut String),
    Single(&'a mut Value),
    Many(VecDeque<RecursiveJsonStringIter<'a>>),
}

struct RecursiveJsonStringIter<'a> {
    inner: RecursiveJsonStringIterInner<'a>,
}

impl<'a> RecursiveJsonStringIter<'a> {
    fn new(value: &'a mut Value) -> Self {
        Self {
            inner: RecursiveJsonStringIterInner::Single(value),
        }
    }
}

fn find_first_in_many<'a>(many: &mut VecDeque<RecursiveJsonStringIter<'a>>) -> Option<&'a mut String> {
    let mut output = None;
    while output.is_none() && !many.is_empty() {
        output = many.front_mut().unwrap().next();
        if output.is_none() {
            many.pop_front();
        }
    }
    output
}

impl<'a> Iterator for RecursiveJsonStringIter<'a> {
    type Item = &'a mut String;

    fn next(&mut self) -> Option<Self::Item> {
        let mut inner = RecursiveJsonStringIterInner::Exhausted;
        std::mem::swap(&mut inner, &mut self.inner);

        let mut handle_many = |mut many: VecDeque<RecursiveJsonStringIter<'a>>| {
            let output = find_first_in_many(&mut many);
            if !many.is_empty() {
                self.inner = RecursiveJsonStringIterInner::Many(many);
            }
            output
        };

        match inner {
            RecursiveJsonStringIterInner::Exhausted => None,
            RecursiveJsonStringIterInner::Single(value) => {
                match value {
                    Value::Array(values) => {
                        let many = values.iter_mut()
                            .map(|value| Self::new(value))
                            .collect::<VecDeque<_>>();
                        handle_many(many)
                    },
                    Value::Object(values) => {
                        let many = values.values_mut()
                            .map(|value| Self::new(value))
                            .collect::<VecDeque<_>>();
                        handle_many(many)
                    }
                    Value::String(value) => Some(value),
                    // The other values don't represent a string, so consider this iter exhausted
                    _ => None,
                }
            },
            RecursiveJsonStringIterInner::Many(many) => {
                handle_many(many)
            }
        }
    }
}

/// Raw response HEOS metadata.
#[derive(Deserialize, Debug)]
pub struct RawResponseHeos {
    /// The full command that produced this response.
    ///
    /// For commands, this will be of the format "\<group\>/\<command\>".
    ///
    /// For events, this will be of the format "event/\<event\>".
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

    pub fn percent_decode(&mut self) {
        if let Some(payload) = &mut self.payload {
            for str_value in RecursiveJsonStringIter::new(payload) {
                let bytes = urlencoding::decode_binary(str_value.as_bytes());
                *str_value = String::from_utf8_lossy(&bytes).into_owned();
            }
        }

        if let Some(options) = &mut self.options {
            for str_value in RecursiveJsonStringIter::new(options) {
                let bytes = urlencoding::decode_binary(str_value.as_bytes());
                *str_value = String::from_utf8_lossy(&bytes).into_owned();
            }
        }
    }
}

impl TryFrom<RawResponse> for () {
    type Error = CommandError;

    #[inline]
    fn try_from(_: RawResponse) -> Result<Self, Self::Error> {
        Ok(())
    }
}