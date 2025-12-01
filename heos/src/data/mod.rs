//! Compound data types used or returned by HEOS commands.

use std::fmt::Display;
use std::num::ParseIntError;

pub mod common;
pub mod event;
pub mod group;
pub mod media;
pub mod option;
pub mod player;
pub mod quickselect;
pub mod response;
pub mod queue;
pub mod source;
pub mod system;

macro_rules! id_type {
    (
        $(#[$attr:meta])*
        $v:vis struct $type_name:ident(pub $inner_type:ty);
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(serde::Serialize, serde::Deserialize, educe::Educe, Clone, Copy, PartialEq, Eq, Hash)]
        #[serde(transparent)]
        #[educe(Deref, DerefMut)]
        $v struct $type_name(pub $inner_type);

        // Custom Debug impl so that pretty printing doesn't add unnecessary line breaks
        impl std::fmt::Debug for $type_name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!(stringify!($type_name), "({:?})"), self.0)
            }
        }

        impl std::fmt::Display for $type_name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl From<$inner_type> for $type_name {
            #[inline]
            fn from(value: $inner_type) -> Self {
                Self(value)
            }
        }

        impl core::str::FromStr for $type_name {
            type Err = core::num::ParseIntError;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let value: $inner_type = s.parse()?;
                Ok(Self(value))
            }
        }
    };
}
pub(in crate::data) use id_type;

/// Error that occurs when trying to create a bounded type with a value that is out of bounds.
#[derive(thiserror::Error, Debug)]
#[error("'{value}' is out of bounds: {min}..={max}")]
pub struct NumberOutOfBoundsError<N: Display> {
    value: N,
    min: N,
    max: N,
}

/// Error that occurs when failing to parse a bounded type from a string.
#[derive(thiserror::Error, Debug)]
pub enum ParseBoundedNumberError<N: Display> {
    /// The parse failed to find a valid numeric type.
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),
    /// The parsed number is out of bounds.
    #[error(transparent)]
    OutOfBounds(#[from] NumberOutOfBoundsError<N>),
}

macro_rules! bounded_number_type {
    (
        $(#[$attr:meta])*
        $v:vis struct $type_name:ident($subtype:ty, $min:literal..=$max:literal);
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(serde::Serialize, serde::Deserialize, educe::Educe, Clone, Copy, PartialEq, Eq, Hash)]
        #[serde(transparent)]
        #[educe(Deref)]
        $v struct $type_name($subtype);

        // Custom Debug impl so that pretty printing doesn't add unnecessary line breaks
        impl std::fmt::Debug for $type_name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, concat!(stringify!($type_name), "({:?})"), self.0)
            }
        }

        impl std::fmt::Display for $type_name {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        impl $type_name {
            /// Minimum valid value for this bounded type, inclusive.
            pub const MIN: $subtype = $min;
            /// Maximum valid value for this bounded type, inclusive.
            pub const MAX: $subtype = $max;

            /// Create a new bounded type, checking to make sure the bounds are met.
            ///
            /// # Errors
            ///
            /// Errors if the given `value` is outside of the valid bounds.
            #[inline]
            pub fn new(value: impl Into<$subtype>) -> Result<Self, crate::data::NumberOutOfBoundsError<$subtype>> {
                let value = value.into();
                #[allow(unused_comparisons)]
                if value <= $max && value >= $min {
                    Ok(Self(value))
                } else {
                    Err(crate::data::NumberOutOfBoundsError {
                        value,
                        min: $min,
                        max: $max,
                    })
                }
            }

            /// Add to this bounded type, saturating at [Self::MAX] if the result would be higher.
            #[inline]
            pub fn saturating_add(self, rhs: impl Into<$subtype>) -> Self {
                Self(self.0.saturating_add(rhs.into()).min(Self::MAX))
            }

            /// Subtract from this bounded type, saturating at [Self::MIN] if the result would be
            /// lower.
            #[inline]
            pub fn saturating_sub(self, rhs: impl Into<$subtype>) -> Self {
                Self(self.0.saturating_sub(rhs.into()).max(Self::MIN))
            }
        }

        impl TryFrom<$subtype> for $type_name {
            type Error = crate::data::NumberOutOfBoundsError<$subtype>;

            #[inline]
            fn try_from(value: $subtype) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$type_name> for $subtype {
            fn from(value: $type_name) -> Self {
                value.0
            }
        }

        impl core::str::FromStr for $type_name {
            type Err = crate::data::ParseBoundedNumberError<$subtype>;

            #[inline]
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let value: $subtype = s.parse()?;
                Ok(Self::new(value)?)
            }
        }
    };
}
pub(in crate::data) use bounded_number_type;

macro_rules! impl_enum_string_conversions {
    ($type_name:ident) => {
        impl core::convert::TryFrom<String> for $type_name {
            type Error = strum::ParseError;

            #[inline]
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::from_str(&value)
            }
        }

        impl From<$type_name> for String {
            #[inline]
            fn from(value: $type_name) -> Self {
                value.to_string()
            }
        }
    };
}
pub(in crate::data) use impl_enum_string_conversions;

macro_rules! impl_try_from_qs {
    ($type_name:ident, $qs_name:literal) => {
        impl<'a> TryFrom<&'a qstring::QString> for $type_name {
            type Error = crate::command::CommandError;

            #[inline]
            fn try_from(qs: &'a qstring::QString) -> Result<Self, Self::Error> {
                qs.get($qs_name)
                    .ok_or(crate::command::CommandError::response_missing_field(format!(
                        "message.{}", $qs_name,
                    )))?
                    .parse()
                    .map_err(|err| crate::command::CommandError::MalformedResponse(format!(
                        "could not parse '{}': {err:?}", $qs_name,
                    )))
            }
        }

        impl TryFrom<qstring::QString> for $type_name {
            type Error = crate::command::CommandError;

            #[inline]
            fn try_from(qs: qstring::QString) -> Result<Self, Self::Error> {
                Self::try_from(&qs)
            }
        }
    };
}
pub(in crate::data) use impl_try_from_qs;

macro_rules! impl_try_from_response_payload {
    ($type_name:ty) => {
        impl TryFrom<crate::data::response::RawResponse> for $type_name {
            type Error = crate::command::CommandError;

            #[inline]
            fn try_from(response: crate::data::response::RawResponse) -> Result<Self, Self::Error> {
                let payload = response.payload
                    .ok_or(crate::command::CommandError::response_missing_field("payload"))?;
                let value = serde_json::from_value::<$type_name>(payload)?;
                Ok(value)
            }
        }
    };
}
pub(in crate::data) use impl_try_from_response_payload;

macro_rules! impl_try_from_response_qs {
    ($type_name:ident) => {
        impl TryFrom<crate::data::response::RawResponse> for $type_name {
            type Error = crate::command::CommandError;

            #[inline]
            fn try_from(response: crate::data::response::RawResponse) -> Result<Self, Self::Error> {
                let qs = qstring::QString::from(response.heos.message.as_str());
                $type_name::try_from(qs)
            }
        }
    };
}
pub(in crate::data) use impl_try_from_response_qs;

pub(crate) mod maybe_range {
    use std::ops::RangeInclusive;
    use serde::{Serialize, Serializer};

    pub fn serialize<S: Serializer>(range: &Option<RangeInclusive<usize>>, s: S) -> Result<S::Ok, S::Error> {
        match range {
            Some(range) => format!("{},{}", range.start(), range.end()).serialize(s),
            other => other.serialize(s),
        }
    }
}

mod maybe_url {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;
    use std::fmt::{Display, Formatter};
    use url::Url;

    pub fn fmt(url: &Option<Url>, f: &mut Formatter) -> fmt::Result {
        match url {
            Some(url) => Display::fmt(url, f),
            None => write!(f, ""),
        }
    }

    pub fn serialize<S: Serializer>(maybe_url: &Option<Url>, s: S) -> Result<S::Ok, S::Error> {
        match maybe_url {
            Some(url) => url.serialize(s),
            None => "".serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Url>, D::Error> {
        let url_str = String::deserialize(d)?;
        if url_str.is_empty() {
            Ok(None)
        } else {
            let url = Url::parse(&url_str)
                .map_err(serde::de::Error::custom)?;
            Ok(Some(url))
        }
    }
}

mod yes_no {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(yes_no: &bool, s: S) -> Result<S::Ok, S::Error> {
        match yes_no {
            true => "yes".serialize(s),
            false => "no".serialize(s),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
        let yes_no_str = String::deserialize(d)?;
        match yes_no_str.as_str() {
            "yes" => Ok(true),
            "no" => Ok(false),
            unknown => Err(serde::de::Error::custom(format!("unknown yes/no value: '{unknown}'"))),
        }
    }
}