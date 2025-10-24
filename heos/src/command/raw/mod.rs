/// Raw untyped commands.
///
/// All typed [Commands](Command) can be serialized into a [RawCommand], which can be further
/// serialized into a pure string. See [RawCommand] for more.

use ahash::HashMap;
use serde::Serialize;
use std::fmt::{Display, Formatter};

use crate::command::raw::ser::to_raw_command;
use crate::command::Command;

mod ser;

pub use ser::Error as SerializeError;

/// Raw untyped command.
///
/// This is an intermediary type that exists between typed [Commands](Command) and pure string
/// command representations. The general serialization flow looks like so:
///
/// ```
/// use heos::command::raw::RawCommand;
/// # use heos::command::raw::SerializeError;
/// use heos::command::system::CheckAccount;
///
/// let check_account_cmd = CheckAccount::default();
/// let raw_cmd = RawCommand::from_command(&check_account_cmd)?;
/// let str_cmd = raw_cmd.to_string();
/// #
/// # Ok::<(), SerializeError>(())
/// ```
///
/// In addition, RawCommands can be built from scratch using a builder-style pattern, in the event
/// a typed [Command] doesn't exist or is not fully supported for the specific command needed.
///
/// ```
/// use heos::command::raw::RawCommand;
///
/// let mut raw_cmd = RawCommand::new("system", "sign_in");
/// raw_cmd
///     .param("username", "MyUsername")
///     .param("password", "MyPassword");
///
/// // Or, to go all the way to a string in one go:
/// let str_cmd = RawCommand::new("system", "sign_in")
///     .param("username", "MyUsername")
///     .param("password", "MyPassword")
///     .to_string();
/// ```
#[derive(Debug, Clone)]
pub struct RawCommand {
    group: String,
    command: String,
    params: HashMap<String, String>,
}

impl RawCommand {
    /// Create a new RawCommand with a specific group/command combination.
    ///
    /// ```
    /// use heos::command::raw::RawCommand;
    ///
    /// let check_account_cmd = RawCommand::new("system", "register_for_change_events");
    /// ```
    #[inline]
    pub fn new(
        group: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self {
            group: group.into(),
            command: command.into(),
            params: HashMap::default(),
        }
    }

    /// Create a new RawCommand with a specific group/command combination and parameters.
    ///
    /// Useful for if you already have parameters stored in e.g. a hash map.
    ///
    /// ```
    /// use heos::command::raw::RawCommand;
    /// use std::collections::HashMap;
    ///
    /// let mut params = HashMap::new();
    /// params.insert("username".to_string(), "MyUsername".to_string());
    /// params.insert("password".to_string(), "MyPassword".to_string());
    /// let sign_in_cmd = RawCommand::new_with_params("system", "sign_in", params);
    /// ```
    #[inline]
    pub fn new_with_params(
        group: impl Into<String>,
        command: impl Into<String>,
        params: impl IntoIterator<Item=(String, String)>,
    ) -> Self {
        let params = params.into_iter()
            .collect::<HashMap<String, String>>();
        Self {
            group: group.into(),
            command: command.into(),
            params,
        }
    }

    /// Add a parameter to an existing RawCommand.
    ///
    /// ```
    /// use heos::command::raw::RawCommand;
    ///
    /// let mut raw_cmd = RawCommand::new("system", "sign_in");
    /// raw_cmd
    ///     .param("username", "MyUsername")
    ///     .param("password", "MyPassword");
    /// ```
    #[inline]
    pub fn param(
        &mut self,
        attribute: impl Into<String>,
        value: impl Into<String>,
    ) -> &mut Self {
        self.params.insert(attribute.into(), value.into());
        self
    }

    /// Create a RawCommand from a typed [Command].
    ///
    /// ```
    /// use heos::command::raw::RawCommand;
    /// # use heos::command::raw::SerializeError;
    /// use heos::command::system::CheckAccount;
    ///
    /// let check_account_cmd = CheckAccount::default();
    /// let raw_cmd = RawCommand::from_command(&check_account_cmd)?;
    /// #
    /// # Ok::<(), SerializeError>(())
    /// ```
    #[inline]
    pub fn from_command<C: Command + Serialize>(cmd: &C) -> Result<Self, SerializeError> {
        to_raw_command(cmd)
    }
}

impl Display for RawCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "heos://{}/{}", &self.group, &self.command)?;
        let mut first = true;
        for (attribute, value) in &self.params {
            // TODO: Escape special characters
            if first {
                write!(f, "?{attribute}={value}")?;
            } else {
                write!(f, "&{attribute}={value}")?;
            }
            first = false;
        }
        Ok(())
    }
}