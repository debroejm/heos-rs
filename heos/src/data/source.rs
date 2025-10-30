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
use std::str::FromStr;
use strum::EnumString;
use url::Url;

use super::*;
use crate::command::CommandError;
use crate::data::option::impl_has_options;
use crate::data::response::RawResponse;

// TODO: Turn this into an enum?
id_type! {
    /// ID representing a specific music source.
    ///
    /// These IDs are globally static, and pre-set per source.
    pub struct SourceId(pub i64);
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
#[derive(Deserialize, Educe)]
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

/// Criteria to use to search by.
///
/// This is used by some services when searching in order to determine what media types are being
/// searched. The `name` corresponds to the type of media, and the `criteria` is the media type ID.
/// For example, if a search criteria result has the name "Artist", then the `criteria` can be used
/// with [Search](crate::command::browse::Search) commands to search for artists.
#[derive(Deserialize, Debug)]
pub struct SearchCriteria {
    /// Name of the criteria.
    ///
    /// This can be e.g. Artist/Album/Track/Station.
    pub name: String,
    /// ID to be used with [Search](crate::command::browse::Search) commands to search for this
    /// media type.
    #[serde(rename = "scid")]
    pub criteria: String,
    /// Do searches of this type support wildcards ('*').
    ///
    /// If this is `None`, assume `false`.
    // TODO: Change this to just be `false` when it is `None`
    #[serde(deserialize_with = "super::maybe_yes_no::deserialize")]
    pub wildcard: Option<bool>,
    /// Are searches of this type directly playable by adding the search results to the queue.
    ///
    /// If this is `None`, assume `false`.
    // TODO: Change this to just be `false` when it is `None`
    #[serde(deserialize_with = "super::maybe_yes_no::deserialize")]
    pub playable: Option<bool>,
    /// If present, this should be prefixed to search strings when searching for media of this type.
    // TODO: Change the name of this field, as it's not actually a container_id
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
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