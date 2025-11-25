//! Data types representing music sources.
//!
//! A music source is anything that can provide music to HEOS players.
//!
//! Built-in music sources include:
//!  * Local USB Media / Local DLNA servers
//!  * HEOS Playlists
//!  * HEOS History
//!  * HEOS aux inputs
//!  * HEOS Favorites
//!
//! As of CLI spec v1.17, these are the 3rd-party music services that HEOS supports through the CLI:
//!  * Pandora
//!  * Rhapsody
//!  * TuneIn
//!  * Deezer
//!  * Napster
//!  * iHeartRadio
//!  * Sirius XM
//!  * Soundcloud
//!  * Tidal
//!  * Amazon Music

use educe::Educe;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use strum::EnumString;
use url::Url;

use super::*;
use crate::command::CommandError;
use crate::data::option::impl_has_options;
use crate::data::response::RawResponse;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(from = "i64", into = "i64")]
pub enum SourceId {
    Pandora,
    Rhapsody,
    TuneIn,
    Spotify,
    Deezer,
    Napster,
    IHeartRadio,
    SiriusXm,
    Soundcloud,
    Tidal,
    AmazonMusic,
    Moodmix,
    QQMusic,
    Qobuz,
    LocalUsbOrDlna,
    HeosPlaylists,
    HeosHistory,
    HeosAuxInputs,
    HeosFavorites,
    Unknown(i64),
}

impl Debug for SourceId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SourceId({:?})", i64::from(*self))
    }
}

impl Display for SourceId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&i64::from(*self), f)
    }
}

impl From<i64> for SourceId {
    #[inline]
    fn from(value: i64) -> Self {
        match value {
            1 => Self::Pandora,
            2 => Self::Rhapsody,
            3 => Self::TuneIn,
            4 => Self::Spotify,
            5 => Self::Deezer,
            6 => Self::Napster,
            7 => Self::IHeartRadio,
            8 => Self::SiriusXm,
            9 => Self::Soundcloud,
            10 => Self::Tidal,
            13 => Self::AmazonMusic,
            15 => Self::Moodmix,
            18 => Self::QQMusic,
            30 => Self::Qobuz,
            1024 => Self::LocalUsbOrDlna,
            1025 => Self::HeosPlaylists,
            1026 => Self::HeosHistory,
            1027 => Self::HeosAuxInputs,
            1028 => Self::HeosFavorites,
            value => Self::Unknown(value),
        }
    }
}

impl From<SourceId> for i64 {
    #[inline]
    fn from(value: SourceId) -> Self {
        match value {
            SourceId::Pandora => 1,
            SourceId::Rhapsody => 2,
            SourceId::TuneIn => 3,
            SourceId::Spotify => 4,
            SourceId::Deezer => 5,
            SourceId::Napster => 6,
            SourceId::IHeartRadio => 7,
            SourceId::SiriusXm => 8,
            SourceId::Soundcloud => 9,
            SourceId::Tidal => 10,
            SourceId::AmazonMusic => 13,
            SourceId::Moodmix => 15,
            SourceId::QQMusic => 18,
            SourceId::Qobuz => 30,
            SourceId::LocalUsbOrDlna => 1024,
            SourceId::HeosPlaylists => 1025,
            SourceId::HeosHistory => 1026,
            SourceId::HeosAuxInputs => 1027,
            SourceId::HeosFavorites => 1028,
            SourceId::Unknown(value) => value,
        }
    }
}

impl FromStr for SourceId {
    type Err = ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: i64 = s.parse()?;
        Ok(value.into())
    }
}

impl SourceId {
    #[inline]
    pub fn cli_browse(&self) -> bool {
        match self {
            Self::Pandora |
            Self::Rhapsody |
            Self::TuneIn |
            Self::Deezer |
            Self::Napster |
            Self::IHeartRadio |
            Self::SiriusXm |
            Self::Soundcloud |
            Self::Tidal |
            Self::AmazonMusic |
            Self::LocalUsbOrDlna |
            Self::HeosPlaylists |
            Self::HeosHistory |
            Self::HeosAuxInputs |
            Self::HeosFavorites => true,
            _ => false,
        }
    }

    #[inline]
    pub fn cli_search(&self) -> bool {
        match self {
            Self::Rhapsody |
            Self::TuneIn |
            Self::Deezer |
            Self::Napster |
            Self::Soundcloud |
            Self::Tidal |
            Self::Qobuz |
            Self::LocalUsbOrDlna |
            Self::HeosFavorites => true,
            _ => false,
        }
    }

    #[inline]
    pub fn cli_new_station(&self) -> bool {
        match self {
            Self::Pandora |
            Self::Rhapsody |
            Self::TuneIn |
            Self::Deezer |
            Self::Napster |
            Self::IHeartRadio |
            Self::Soundcloud |
            Self::Tidal => true,
            _ => false,
        }
    }
}

/// Broad category that a source belongs to.
#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "snake_case")]
pub enum SourceType {
    /// Third-party music services.
    MusicService,
    /// Built-in HEOS services.
    HeosService,
    /// Built-in HEOS servers.
    HeosServer,
    /// DLNA servers.
    DlnaServer,
}
impl_enum_string_conversions!(SourceType);

/// Status of source availability.
#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum SourceAvailable {
    /// The source is available to play from.
    ///
    /// For music services, this means that an account is logged-in and associated with the music
    /// service for it to be usable.
    True,
    /// The source is NOT available to play from.
    False,
}
impl_enum_string_conversions!(SourceAvailable);

/// Information about a specific music source.
#[derive(Serialize, Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct SourceInfo {
    /// Name of the source.
    pub name: String,
    /// URL of an image that can be used to represent the source.
    #[educe(Debug(method(std::fmt::Display::fmt)))]
    pub image_url: Url,
    /// Broad category the source belongs to.
    #[serde(rename = "type")]
    pub source_type: SourceType,
    /// ID of the source.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// Whether the source is available to play music from.
    pub available: SourceAvailable,
    /// Username associated with the source, if the source is a music service and there is an
    /// account logged-in with the music service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_username: Option<String>,
}
impl_try_from_response_payload!(SourceInfo);
impl_try_from_response_payload!(Vec<SourceInfo>);

/// Information about a media item retrieved for HEOS servers or services.
///
/// This usually represents an input source where actual track/artist/album metadata can't be
/// determined, such as from an aux input.
#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct HeosSourceItem {
    /// Name of the media item.
    pub name: String,
    /// URL to an image representing the media item, if it exists.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// ID of the source the media item is from.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
}

/// Common data associated with many media items.
#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct MediaSourceItem {
    /// Is the media item directly playable?
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub playable: bool,
    /// URL to an image representing the media item, if it exists.
    ///
    /// If this is `None` and the media item is a track/artist/album, it may be able to be retrieved
    /// via the [GetAlbumMetadata](crate::command::browse::GetAlbumMetadata) command.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// ID representing this media item, if it exists.
    // TODO: Need to investigate which media items this is valid for, as it may be better to move it
    //  out of the common metadata.
    #[serde(rename = "mid")]
    pub media_id: Option<String>,
}

/// Information about an artist.
#[derive(Deserialize, Debug)]
pub struct ArtistSourceItem {
    /// Name of the artist.
    pub name: String,
    /// ID representing the artist's "container" (all music by said artist).
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

/// Information about an album.
#[derive(Deserialize, Debug)]
pub struct AlbumSourceItem {
    /// Name of the album.
    pub name: String,
    /// Name of the album's artist.
    pub artist: String,
    /// ID representing the album.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

/// Information about a track.
#[derive(Deserialize, Debug)]
pub struct SongSourceItem {
    /// Name of the track.
    pub name: String,
    /// Name of the track's artist.
    pub artist: String,
    /// Name of the album the track is from.
    ///
    /// This may be blank if the track doesn't belong to an album.
    pub album: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

/// Information about a generic music container.
#[derive(Deserialize, Debug)]
pub struct ContainerSourceItem {
    /// Name of the container.
    pub name: String,
    /// ID of the container.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

/// Information about a music station.
#[derive(Deserialize, Debug)]
pub struct StationSourceItem {
    /// Name of the station.
    pub name: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

/// Information about a music genre, used as a container.
#[derive(Deserialize, Debug)]
pub struct GenreSourceItem {
    /// Name of the genre.
    pub name: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

/// A single item yielded from a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceItem {
    /// Items yielded by [HEOS services](SourceType::HeosService).
    HeosService(HeosSourceItem),
    /// Items yielded by [HEOS servers](SourceType::HeosServer).
    HeosServer(HeosSourceItem),
    /// A music artist.
    Artist(ArtistSourceItem),
    /// An album of tracks.
    Album(AlbumSourceItem),
    /// A single track.
    Song(SongSourceItem),
    /// A generic container of music.
    ///
    /// Could be e.g. a folder in a Windows Media Share.
    Container(ContainerSourceItem),
    /// A music station.
    Station(StationSourceItem),
    /// A genre of music being used as a container.
    Genre(GenreSourceItem),
}
impl_has_options!(Vec<SourceItem>, "browse");
impl_try_from_response_payload!(Vec<SourceItem>);

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(from = "i64", into = "i64")]
pub enum CriteriaId {
    Artist,
    Album,
    Track,
    Station,
    Shows,
    Playlist,
    Accounts,
    Unknown(i64),
}

impl Debug for CriteriaId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CriteriaId({:?})", i64::from(*self))
    }
}

impl Display for CriteriaId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&i64::from(*self), f)
    }
}

impl From<i64> for CriteriaId {
    #[inline]
    fn from(value: i64) -> Self {
        match value {
            1 => Self::Artist,
            2 => Self::Album,
            3 => Self::Track,
            4 => Self::Station,
            5 => Self::Shows,
            6 => Self::Playlist,
            7 => Self::Accounts,
            value => Self::Unknown(value),
        }
    }
}

impl From<CriteriaId> for i64 {
    #[inline]
    fn from(value: CriteriaId) -> Self {
        match value {
            CriteriaId::Artist => 1,
            CriteriaId::Album => 2,
            CriteriaId::Track => 3,
            CriteriaId::Station => 4,
            CriteriaId::Shows => 5,
            CriteriaId::Playlist => 6,
            CriteriaId::Accounts => 7,
            CriteriaId::Unknown(value) => value,
        }
    }
}

impl FromStr for CriteriaId {
    type Err = ParseIntError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: i64 = s.parse()?;
        Ok(value.into())
    }
}

/// Criteria to use to search by.
///
/// This is used by some services when searching in order to determine what media types are being
/// searched. The `name` corresponds to the type of media, and the `criteria` is the media type ID.
/// For example, if a search criteria result has the name "Artist", then the `criteria` can be used
/// with [Search](crate::command::browse::Search) commands to search for artists.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchCriteria {
    /// Name of the criteria.
    ///
    /// This can be e.g. Artist/Album/Track/Station.
    pub name: String,
    /// ID to be used with [Search](crate::command::browse::Search) commands to search for this
    /// media type.
    #[serde(rename = "scid")]
    pub criteria: CriteriaId,
    /// Do searches of this type support wildcards ('*').
    #[serde(default)]
    #[serde(with = "super::yes_no")]
    pub wildcard: bool,
    /// Are searches of this type directly playable by adding the search results to the queue.
    #[serde(default)]
    #[serde(with = "super::yes_no")]
    pub playable: bool,
    /// If present, this should be prefixed to search strings when searching for media of this type.
    #[serde(rename = "cid")]
    #[serde(default)]
    pub search_prefix: Option<String>,
}
impl_try_from_response_payload!(Vec<SearchCriteria>);

/// Results of using a [Search](crate::command::browse::Search) command.
#[derive(Debug)]
pub struct SearchResults {
    /// How many total items are available in the container being searched.
    ///
    /// If this is larger than the size of `source_items`, the
    /// [Search](crate::command::browse::Search) command needs to be repeated until all results are
    /// retrieved.
    pub count: usize,
    /// Collection of search results.
    ///
    /// This will not be larger than the maximum range specified in the
    /// [Search](crate::command::browse::Search) command, or the default maximum if a range is not
    /// specified.
    pub source_items: Vec<SourceItem>,
}
impl_has_options!(SearchResults, "browse");

impl TryFrom<RawResponse> for SearchResults {
    type Error = CommandError;

    #[inline]
    fn try_from(response: RawResponse) -> Result<Self, Self::Error> {
        let qs = qstring::QString::from(response.heos.message.as_str());
        let count: usize = qs.get("count")
            .ok_or(CommandError::response_missing_field("message.count"))?
            .parse()
            .map_err(|err| CommandError::MalformedResponse(format!(
                "could not parse 'count': {err:?}",
            )))?;

        let source_items = Vec::<SourceItem>::try_from(response)?;

        Ok(SearchResults {
            count,
            source_items,
        })
    }
}

/// Metadata about a particular album's art.
///
/// This contains e.g. the album art URL for an album.
#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct AlbumImageMetadata {
    /// URL to the album's art.
    #[educe(Debug(method(std::fmt::Display::fmt)))]
    pub image_url: Url,
    /// Image width.
    pub width: usize,
}

/// Metadata about a particular album.
#[derive(Deserialize, Debug)]
pub struct AlbumMetadata {
    /// ID of the album.
    pub album_id: String,
    /// One or more album art images associated with the album.
    ///
    /// These may be the same image at multiple sizes, so the image with the most optimum size
    /// should be chosen.
    pub images: Vec<AlbumImageMetadata>,
}
impl_try_from_response_payload!(Vec<AlbumMetadata>);