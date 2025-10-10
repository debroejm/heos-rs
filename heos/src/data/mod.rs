use std::fmt::Display;
use std::num::ParseIntError;

pub mod common;
pub mod event;
pub mod group;
pub mod option;
pub mod player;
pub mod quickselect;
pub mod response;
pub mod song;
pub mod source;
pub mod system;

macro_rules! id_type {
    ($type_name:ident) => {
        #[repr(transparent)]
        #[derive(serde::Serialize, serde::Deserialize, educe::Educe, Clone, Copy, PartialEq, Eq, Hash)]
        #[serde(transparent)]
        #[educe(Deref, DerefMut)]
        pub struct $type_name(pub i64);

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

        impl From<i64> for $type_name {
            #[inline]
            fn from(value: i64) -> Self {
                Self(value)
            }
        }
    };
}
pub(in crate::data) use id_type;

#[derive(thiserror::Error, Debug)]
#[error("'{value}' is out of bounds: {min}..={max}")]
pub struct NumberOutOfBoundsError<N: Display> {
    value: N,
    min: N,
    max: N,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseBoundedNumberError<N: Display> {
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),
    #[error(transparent)]
    OutOfBounds(#[from] NumberOutOfBoundsError<N>),
}

macro_rules! bounded_number_type {
    ($type_name:ident, $subtype:ty, $min:literal..=$max:literal) => {
        #[repr(transparent)]
        #[derive(serde::Serialize, serde::Deserialize, educe::Educe, Clone, Copy, PartialEq, Eq)]
        #[serde(transparent)]
        #[educe(Deref)]
        pub struct $type_name($subtype);

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
            pub const MIN: $subtype = $min;
            pub const MAX: $subtype = $max;

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

            #[inline]
            pub fn saturating_add(self, rhs: impl Into<$subtype>) -> Self {
                Self(self.0.saturating_add(rhs.into()).min(Self::MAX))
            }

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
    use serde::{Deserialize, Deserializer};
    use std::fmt;
    use std::fmt::{Display, Formatter};
    use url::Url;

    pub fn fmt(url: &Option<Url>, f: &mut Formatter) -> fmt::Result {
        match url {
            Some(url) => Display::fmt(url, f),
            None => write!(f, ""),
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
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
        let yes_no_str = String::deserialize(d)?;
        match yes_no_str.as_str() {
            "yes" => Ok(true),
            "no" => Ok(false),
            unknown => Err(serde::de::Error::custom(format!("unknown yes/no value: '{unknown}'"))),
        }
    }
}

mod maybe_yes_no {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<bool>, D::Error> {
        let yes_no_str = match Option::<String>::deserialize(d)? {
            Some(yes_no_str) => yes_no_str,
            None => return Ok(None),
        };
        match yes_no_str.as_str() {
            "yes" => Ok(Some(true)),
            "no" => Ok(Some(false)),
            unknown => Err(serde::de::Error::custom(format!("unknown yes/no value: '{unknown}'"))),
        }
    }
}