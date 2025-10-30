//! Data types relevant to music service-specific options.
//!
//! Different music services can have unique options that can be set or otherwise managed. These
//! data types represent those options, and will usually only apply to certain music services.
//!
//! The following commands can yield responses that contain data wrapped [WithOptions]:
//!  * [GetNowPlayingMedia](crate::command::player::GetNowPlayingMedia)
//!  * [Browse](crate::command::browse::Browse)
//!  * [Search](crate::command::browse::Search)
//!
//! Once valid option types have been discovered via the above commands, they can be set via
//! [SetServiceOption](crate::command::browse::SetServiceOption).

use std::ops::RangeInclusive;
use educe::Educe;
use serde::{Deserialize, Serialize};

use crate::command::CommandError;
use crate::data::maybe_range;
use crate::data::player::PlayerId;
use crate::data::response::RawResponse;

/// Sub-enum for adding a media item to HEOS favorites.
#[derive(Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum AddToHeosFavorites {
    /// Add the currently playing track to favorites.
    NowPlaying {
        /// ID of the player to retrieve the currently playing track from.
        #[serde(rename = "pid")]
        player_id: PlayerId,
    },
    /// Add a result from a [browse](crate::command::browse::Browse) command to favorites.
    Browse {
        /// ID of the media to add.
        #[serde(rename = "mid")]
        media_id: String,
        /// Name of the media to add.
        name: String,
    }
}

/// The service option to set or otherwise manipulate.
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "option")]
pub enum ServiceOption {
    /// Add a track to the user's music service library.
    #[serde(rename = "1")]
    AddTrackToLibrary {
        /// ID of the track.
        #[serde(rename = "mid")]
        media_id: String,
    },
    /// Add an album to the user's music service library.
    #[serde(rename = "2")]
    AddAlbumToLibrary {
        /// ID of the album.
        #[serde(rename = "cid")]
        container_id: String,
    },
    /// Add a station to the user's music service library.
    #[serde(rename = "3")]
    AddStationToLibrary {
        /// ID of the station.
        #[serde(rename = "mid")]
        media_id: String,
    },
    /// Add a playlist to the user's music service library.
    #[serde(rename = "4")]
    AddPlaylistToLibrary {
        /// ID of the playlist.
        #[serde(rename = "cid")]
        container_id: String,
        /// Name of the playlist.
        name: String,
    },
    /// Remove a track from the user's music service library.
    #[serde(rename = "5")]
    RemoveTrackFromLibrary {
        /// ID of the track.
        #[serde(rename = "mid")]
        media_id: String,
    },
    /// Remove an album from the user's music service library.
    #[serde(rename = "6")]
    RemoveAlbumFromLibrary {
        /// ID of the album.
        #[serde(rename = "cid")]
        container_id: String,
    },
    /// Remove a station from the user's music service library.
    #[serde(rename = "7")]
    RemoveStationFromLibrary {
        /// ID of the station.
        #[serde(rename = "mid")]
        media_id: String,
    },
    /// Remove a playlist from the user's music service library.
    #[serde(rename = "8")]
    RemovePlaylistFromLibrary {
        /// ID of the playlist.
        #[serde(rename = "cid")]
        container_id: String,
    },
    /// Thumbs up the now playing track.
    #[serde(rename = "11")]
    ThumbsUp {
        #[serde(rename = "pid")]
        player_id: PlayerId,
    },
    /// Thumbs down the now playing track.
    #[serde(rename = "12")]
    ThumbsDown {
        #[serde(rename = "pid")]
        player_id: PlayerId,
    },
    /// Create a new station by artists/shows/tracks.
    ///
    /// Some services support only artists, while others support other criteria types.
    #[serde(rename = "13")]
    CreateNewStation {
        /// Search string yielded by [Search](crate::command::browse::Search) commands.
        name: String,
        /// Criteria ID yielded by [Search](crate::command::browse::Search) commands.
        #[serde(rename = "scid")]
        criteria: Option<String>,
        /// Optional range to limit search results by.
        #[serde(serialize_with = "maybe_range::serialize")]
        range: Option<RangeInclusive<usize>>,
    },
    /// Add media to HEOS favorites.
    ///
    /// See [AddToHeosFavorites] for more.
    #[serde(rename = "19")]
    AddToHeosFavorites(AddToHeosFavorites),
    /// Remove media from HEOS favorites.
    #[serde(rename = "20")]
    RemoveFromHeosFavorites {
        /// ID of the media to remove.
        #[serde(rename = "mid")]
        media_id: String,
    },
}

#[derive(Deserialize, Debug, Clone)]
struct ServiceOptionTypeRaw {
    id: i64,
    // TODO: There may be more data that needs to be extracted
}

/// Types of service options that can be set.
///
/// This enum represents types only, but not actual service option data. See [ServiceOption] for a
/// full representation.
#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "ServiceOptionTypeRaw")]
pub enum ServiceOptionType {
    /// See [ServiceOption::AddTrackToLibrary].
    AddTrackToLibrary,
    /// See [ServiceOption::AddAlbumToLibrary].
    AddAlbumToLibrary,
    /// See [ServiceOption::AddStationToLibrary].
    AddStationToLibrary,
    /// See [ServiceOption::AddPlaylistToLibrary].
    AddPlaylistToLibrary,
    /// See [ServiceOption::RemoveTrackFromLibrary].
    RemoveTrackFromLibrary,
    /// See [ServiceOption::RemoveAlbumFromLibrary].
    RemoveAlbumFromLibrary,
    /// See [ServiceOption::RemoveStationFromLibrary].
    RemoveStationFromLibrary,
    /// See [ServiceOption::RemovePlaylistFromLibrary].
    RemovePlaylistFromLibrary,
    /// See [ServiceOption::ThumbsUp].
    ThumbsUp,
    /// See [ServiceOption::ThumbsDown].
    ThumbsDown,
    /// See [ServiceOption::CreateNewStation].
    CreateNewStation,
    /// See [ServiceOption::AddToHeosFavorites].
    AddToHeosFavorites,
    /// See [ServiceOption::RemoveFromHeosFavorites].
    RemoveFromHeosFavorites,
    /// Indicates that the response contains a playable container.
    ///
    /// Used specifically with the [Browse](crate::command::browse::Browse) command for services
    /// that can't determine if a container is playable until the container itself is browsed.
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

/// Marker trait indicating a data type can have [service options](ServiceOption) associated with it.
///
/// This trait also describes how to retrieve service options from a [RawResponse].
pub trait HasOptions {
    /// The top-level key to query for option data in a [RawResponse].
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

/// Wrapper around a data type that can have [service options](ServiceOption) associated with it.
///
/// This struct bundles both the original data and the possible options, and allows dereferencing
/// into the original data for ease of use.
#[derive(Educe, Debug, Clone)]
#[educe(Deref, DerefMut)]
pub struct WithOptions<T> {
    /// Original data.
    #[educe(Deref, DerefMut)]
    pub value: T,
    /// Any valid service options for bundled data.
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