use serde::Serialize;
use std::fmt::Debug;

use crate::command::raw::SerializeError;
use crate::data::response::RawResponse;

pub mod browse;
pub mod group;
pub mod player;
pub mod raw;
pub mod system;

pub trait Command: Serialize {
    const GROUP: &'static str;
    const COMMAND: &'static str;
    type Response: TryFrom<RawResponse, Error=CommandError>;
}

macro_rules! impl_command {
    ($cmd_ident:ident, $group:literal, $command:literal, $response_type:ty) => {
        impl crate::command::Command for $cmd_ident {
            const GROUP: &'static str = $group;
            const COMMAND: &'static str = $command;
            type Response = $response_type;
        }
    };
}
pub(in crate::command) use impl_command;

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("Command not recognized")]
    UnrecognizedCommand,
    #[error("ID not valid")]
    InvalidId,
    #[error("Command arguments are invalid")]
    InvalidArguments,
    #[error("Requested data not available")]
    DataNotAvailable,
    #[error("Resource currently not available")]
    ResourceNotAvailable,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Command could not be executed")]
    CommandNotExecuted,
    #[error("User not logged in")]
    UserNotLoggedIn,
    #[error("Parameter out of range")]
    ParamOutOfRange,
    #[error("User not found")]
    UserNotFound,
    #[error("Internal system error")]
    InternalError,
    #[error("System error (syserrno={0})")]
    SystemError(i64),
    #[error("Busy processing previous command")]
    ProcessingPreviousCommand,
    #[error("Media cannot be played")]
    CannotPlay,
    #[error("Option not supported")]
    NotSupported,
    #[error("Too many commands in message queue")]
    CommandQueueFull,
    #[error("Reached skip limit")]
    SkipLimit,
    #[error("Unknown error ({eid}): {text}")]
    Unknown {
        eid: i64,
        text: String,
    },
    #[error("Response is malformed: {0}")]
    MalformedResponse(String),
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to serialize into raw command: {0}")]
    SerializeError(#[from] SerializeError),
}

impl CommandError {
    pub fn from_message(msg: impl AsRef<str>) -> Self {
        let qs = qstring::QString::from(msg.as_ref());
        let eid = match qs.get("eid") {
            Some(eid) => eid,
            None => return Self::response_missing_field("heos.message.eid"),
        };
        let eid: i64 = match eid.parse() {
            Ok(eid) => eid,
            Err(err) => return Self::MalformedResponse(format!("Could not parse eid: {err}")),
        };
        match eid {
            1 => Self::UnrecognizedCommand,
            2 => Self::InvalidId,
            3 => Self::InvalidArguments,
            4 => Self::DataNotAvailable,
            5 => Self::ResourceNotAvailable,
            6 => Self::InvalidCredentials,
            7 => Self::CommandNotExecuted,
            8 => Self::UserNotLoggedIn,
            9 => Self::ParamOutOfRange,
            10 => Self::UserNotFound,
            11 => Self::InternalError,
            12 => {
                let syserrno = match qs.get("syserrno") {
                    Some(syserrno) => syserrno,
                    None => return Self::response_missing_field("heos.message.syserrno"),
                };
                let syserrno: i64 = match syserrno.parse() {
                    Ok(syserrno) => syserrno,
                    Err(err) => return Self::MalformedResponse(format!("Could not parse syserrno: {err}")),
                };
                Self::SystemError(syserrno)
            },
            13 => Self::ProcessingPreviousCommand,
            14 => Self::CannotPlay,
            15 => Self::NotSupported,
            16 => Self::CommandQueueFull,
            17 => Self::SkipLimit,
            eid => {
                let text = qs.get("text")
                    .map(|t| t.to_string())
                    .unwrap_or_default();
                Self::Unknown {
                    eid,
                    text,
                }
            },
        }
    }

    #[inline]
    pub fn response_missing_field(name: impl AsRef<str>) -> Self {
        Self::MalformedResponse(format!("Missing field '{}'", name.as_ref()))
    }

    #[inline]
    pub fn response_incorrect_data_type(
        expected_type: impl AsRef<str>,
        actual_value: impl Debug,
    ) -> Self {
        Self::MalformedResponse(format!(
            "Incorrect data type (expected='{}'): {:?}",
            expected_type.as_ref(),
            actual_value,
        ))
    }
}

impl From<serde_json::Error> for CommandError {
    #[inline]
    fn from(value: serde_json::Error) -> Self {
        Self::MalformedResponse(format!("Failed to deserialize JSON: {value}"))
    }
}