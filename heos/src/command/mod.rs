/// Commands that can be sent to HEOS devices.
///
/// These are organized into several modules, broadly categorized by the command 'group' as
/// specified by the HEOS CLI specification. These modules are as follows:
///  * [`system`]: Commands related to managing the HEOS control system as a whole.
///  * [`player`]: Commands related to managing individual player devices.
///  * [`group`]: Commands related to managing groups of HEOS devices.
///  * [`browse`]: Commands related to source-specific browse and search functionality.
///
/// In addition, there is a [`raw`] module that allows users to construct
/// [raw commands](raw::RawCommand) in the event that the provided typed commands are insufficient,
/// although this should be unlikely.

use serde::Serialize;
use std::fmt::Debug;

use crate::command::raw::SerializeError;
use crate::data::response::RawResponse;

pub mod browse;
pub mod group;
pub mod player;
pub mod raw;
pub mod system;

/// Typed Command.
///
/// Specific command representations should implement this in order to define type relations between
/// the command representation and any expected responses.
///
/// Most implementations of this trait should be done by the `heos` crate, and users should not need
/// to implement this trait themselves, but it is possible to do so if e.g. a modified version of
/// some command is needed.
///
/// # Examples
///
/// A command without an expected response type:
/// ```
/// use heos::command::Command;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct MyCommand {}
///
/// impl Command for MyCommand {
///     const GROUP: &'static str = "group";
///     const COMMAND: &'static str = "my_command";
///     type Response = ();
/// }
/// ```
///
/// A command with an expected response type:
/// ```
/// use heos::command::{Command, CommandError};
/// use heos::data::response::RawResponse;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct MyCommand {}
///
/// impl Command for MyCommand {
///     const GROUP: &'static str = "group";
///     const COMMAND: &'static str = "my_command";
///     type Response = MyResponse;
/// }
///
/// struct MyResponse {}
///
/// impl TryFrom<RawResponse> for MyResponse {
///     type Error = CommandError;
///
///     fn try_from(response: RawResponse) -> Result<Self, Self::Error> {
///         // Do actual processing in here
///         Ok(MyResponse {})
///     }
/// }
///
/// ```
pub trait Command: Serialize {
    /// Group the command belongs to.
    const GROUP: &'static str;
    /// Name of the command.
    const COMMAND: &'static str;
    /// Type of response that is expected when the command is sent. For commands that don't care
    /// about the expected response, `()` can be used.
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

/// Errors that can occur when sending commands.
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    /// The command was not recognized as a valid HEOS command.
    #[error("Command not recognized")]
    UnrecognizedCommand,
    /// One or more of the IDs specified in the command was not valid.
    #[error("ID not valid")]
    InvalidId,
    /// One or more command arguments were missing or not valid.
    #[error("Command arguments are invalid")]
    InvalidArguments,
    /// The requested data was not available.
    #[error("Requested data not available")]
    DataNotAvailable,
    /// The requested resource was not currently available.
    #[error("Resource currently not available")]
    ResourceNotAvailable,
    /// The specified credentials were not valid.
    #[error("Invalid credentials")]
    InvalidCredentials,
    /// The command could not be executed.
    #[error("Command could not be executed")]
    CommandNotExecuted,
    /// There is no logged-in user when one is required.
    #[error("User not logged in")]
    UserNotLoggedIn,
    /// One or more of the specified parameters were out of valid range.
    #[error("Parameter out of range")]
    ParamOutOfRange,
    /// A user was not found.
    #[error("User not found")]
    UserNotFound,
    /// An internal error occurred.
    #[error("Internal system error")]
    InternalError,
    /// A system error occurred (this is usually in regard to an external music service).
    #[error("System error (syserrno={0})")]
    SystemError(i64),
    /// The system was busy processing a previous command.
    #[error("Busy processing previous command")]
    ProcessingPreviousCommand,
    /// The specified media could not be played.
    #[error("Media cannot be played")]
    CannotPlay,
    /// An option was specified that is not supported by the relevant music service.
    #[error("Option not supported")]
    NotSupported,
    /// The system command queue is full, and cannot process more commands.
    #[error("Too many commands in message queue")]
    CommandQueueFull,
    /// The user's skip limit was reached (this is music service specific).
    #[error("Reached skip limit")]
    SkipLimit,
    /// An unknown/unrecognized error occurred.
    #[error("Unknown error ({eid}): {text}")]
    Unknown {
        eid: i64,
        text: String,
    },
    /// The command's response was malformed and could not be parsed.
    #[error("Response is malformed: {0}")]
    MalformedResponse(String),
    /// A general [IOError](std::io::Error) occurred.
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    /// The command could not be serialized into a [RawCommand](raw::RawCommand).
    #[error("Failed to serialize into raw command: {0}")]
    SerializeError(#[from] SerializeError),
}

impl CommandError {
    /// Parse a CommandError from a
    /// [response message](field@crate::data::response::RawResponseHeos::message).
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

    /// Create a [MalformedResponse](Self::MalformedResponse) error for a missing field.
    #[inline]
    pub fn response_missing_field(name: impl AsRef<str>) -> Self {
        Self::MalformedResponse(format!("Missing field '{}'", name.as_ref()))
    }

    /// Create a [MalformedResponse](Self::MalformedResponse) error for mismatched data types.
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