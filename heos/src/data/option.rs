use std::ops::RangeInclusive;
use educe::Educe;
use serde::{Deserialize, Serialize};

use crate::command::CommandError;
use crate::data::maybe_range;
use crate::data::player::PlayerId;
use crate::data::response::RawResponse;

#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum AddToHeosFavorites {
    NowPlaying {
        #[serde(rename = "pid")]
        player_id: PlayerId,
    },
    Browse {
        #[serde(rename = "mid")]
        media_id: String,
        name: String,
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "option")]
pub enum ServiceOption {
    #[serde(rename = "1")]
    AddTrackToLibrary {
        #[serde(rename = "mid")]
        media_id: String,
    },
    #[serde(rename = "2")]
    AddAlbumToLibrary {
        #[serde(rename = "cid")]
        container_id: String,
    },
    #[serde(rename = "3")]
    AddStationToLibrary {
        #[serde(rename = "mid")]
        media_id: String,
    },
    #[serde(rename = "4")]
    AddPlaylistToLibrary {
        #[serde(rename = "cid")]
        container_id: String,
        name: String,
    },
    #[serde(rename = "5")]
    RemoveTrackFromLibrary {
        #[serde(rename = "mid")]
        media_id: String,
    },
    #[serde(rename = "6")]
    RemoveAlbumFromLibrary {
        #[serde(rename = "cid")]
        container_id: String,
    },
    #[serde(rename = "7")]
    RemoveStationFromLibrary {
        #[serde(rename = "mid")]
        media_id: String,
    },
    #[serde(rename = "8")]
    RemovePlaylistFromLibrary {
        #[serde(rename = "cid")]
        container_id: String,
    },
    #[serde(rename = "11")]
    ThumbsUp {
        #[serde(rename = "pid")]
        player_id: PlayerId,
    },
    #[serde(rename = "12")]
    ThumbsDown {
        #[serde(rename = "pid")]
        player_id: PlayerId,
    },
    #[serde(rename = "13")]
    CreateNewStation {
        name: String,
        #[serde(rename = "scid")]
        criteria: Option<String>,
        #[serde(serialize_with = "maybe_range::serialize")]
        range: Option<RangeInclusive<usize>>,
    },
    #[serde(rename = "19")]
    AddToHeosFavorites(AddToHeosFavorites),
    #[serde(rename = "20")]
    RemoveFromHeosFavorites {
        #[serde(rename = "mid")]
        media_id: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
struct ServiceOptionTypeRaw {
    id: i64,
    // TODO: There may be more data that needs to be extracted
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "ServiceOptionTypeRaw")]
pub enum ServiceOptionType {
    AddTrackToLibrary,
    AddAlbumToLibrary,
    AddStationToLibrary,
    AddPlaylistToLibrary,
    RemoveTrackFromLibrary,
    RemoveAlbumFromLibrary,
    RemoveStationFromLibrary,
    RemovePlaylistFromLibrary,
    ThumbsUp,
    ThumbsDown,
    CreateNewStation,
    AddToHeosFavorites,
    RemoveFromHeosFavorites,
    PlayableContainer,
}

impl TryFrom<ServiceOptionTypeRaw> for ServiceOptionType {
    type Error = String;

    #[inline]
    fn try_from(value: ServiceOptionTypeRaw) -> Result<Self, Self::Error> {
        match value.id {
            1 => Ok(Self::AddTrackToLibrary),
            2 => Ok(Self::AddAlbumToLibrary),
            3 => Ok(Self::AddStationToLibrary),
            4 => Ok(Self::AddPlaylistToLibrary),
            5 => Ok(Self::RemoveTrackFromLibrary),
            6 => Ok(Self::RemoveAlbumFromLibrary),
            7 => Ok(Self::RemoveStationFromLibrary),
            8 => Ok(Self::RemovePlaylistFromLibrary),
            11 => Ok(Self::ThumbsUp),
            12 => Ok(Self::ThumbsDown),
            13 => Ok(Self::CreateNewStation),
            19 => Ok(Self::AddToHeosFavorites),
            20 => Ok(Self::RemoveFromHeosFavorites),
            21 => Ok(Self::PlayableContainer),
            id => Err(format!("unknown service option type: {id}")),
        }
    }
}

impl<'a> From<&'a ServiceOption> for ServiceOptionType {
    #[inline]
    fn from(value: &'a ServiceOption) -> Self {
        match value {
            ServiceOption::AddTrackToLibrary { .. } => Self::AddTrackToLibrary,
            ServiceOption::AddAlbumToLibrary { .. } => Self::AddAlbumToLibrary,
            ServiceOption::AddStationToLibrary { .. } => Self::AddStationToLibrary,
            ServiceOption::AddPlaylistToLibrary { .. } => Self::AddPlaylistToLibrary,
            ServiceOption::RemoveTrackFromLibrary { .. } => Self::RemoveTrackFromLibrary,
            ServiceOption::RemoveAlbumFromLibrary { .. } => Self::RemoveAlbumFromLibrary,
            ServiceOption::RemoveStationFromLibrary { .. } => Self::RemoveStationFromLibrary,
            ServiceOption::RemovePlaylistFromLibrary { .. } => Self::RemovePlaylistFromLibrary,
            ServiceOption::ThumbsUp { .. } => Self::ThumbsUp,
            ServiceOption::ThumbsDown { .. } => Self::ThumbsDown,
            ServiceOption::CreateNewStation { .. } => Self::CreateNewStation,
            ServiceOption::AddToHeosFavorites(_) => Self::AddToHeosFavorites,
            ServiceOption::RemoveFromHeosFavorites { .. } => Self::RemoveFromHeosFavorites,
        }
    }
}

impl From<ServiceOption> for ServiceOptionType {
    #[inline]
    fn from(value: ServiceOption) -> Self {
        Self::from(&value)
    }
}

pub trait HasOptions {
    const OPTION_KEY: &'static str;
}

macro_rules! impl_has_options {
    ($type_ident:ty, $key:literal) => {
        impl crate::data::option::HasOptions for $type_ident {
            const OPTION_KEY: &'static str = $key;
        }
    };
}
pub(crate) use impl_has_options;

#[derive(Educe, Debug, Clone)]
#[educe(Deref, DerefMut)]
pub struct WithOptions<T> {
    #[educe(Deref, DerefMut)]
    pub value: T,
    pub options: Vec<ServiceOptionType>,
}

impl<T> TryFrom<RawResponse> for WithOptions<T>
where
    T: TryFrom<RawResponse, Error=CommandError> + HasOptions,
{
    type Error = CommandError;

    fn try_from(response: RawResponse) -> Result<Self, Self::Error> {
        let options = if let Some(options) = response.options.clone() {
            let option_groups = match options.as_array() {
                Some(groups) => groups.clone(),
                None => vec![],
            };

            let mut all_options = vec![];
            for group in option_groups {
                if let Some(value) = group.get(T::OPTION_KEY) {
                    let options = serde_json::from_value::<Vec<ServiceOptionType>>(value.clone())?;
                    all_options.extend(options);
                }
            }
            all_options
        } else {
            vec![]
        };

        let value = T::try_from(response)?;

        Ok(WithOptions {
            value,
            options,
        })
    }
}