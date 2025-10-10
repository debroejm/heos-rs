use ahash::HashMap;
use serde::Serialize;
use std::fmt::{Display, Formatter};

use crate::command::raw::ser::to_raw_command;
use crate::command::Command;

mod ser;

pub use ser::Error as SerializeError;
#[derive(Debug, Clone)]
pub struct RawCommand {
    group: String,
    command: String,
    params: HashMap<String, String>,
}

impl RawCommand {
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

    #[inline]
    pub fn new_with_params(
        group: impl Into<String>,
        command: impl Into<String>,
        params: HashMap<String, String>,
    ) -> Self {
        Self {
            group: group.into(),
            command: command.into(),
            params,
        }
    }

    #[inline]
    pub fn param(
        &mut self,
        attribute: impl Into<String>,
        value: impl Into<String>,
    ) {
        self.params.insert(attribute.into(), value.into());
    }

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

