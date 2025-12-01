//! Data types representing various media items.
//!
//! These types provide information about e.g. songs, artist, albums, stations, etc.

use educe::Educe;
use serde::Deserialize;
use url::Url;

use crate::data::impl_try_from_response_payload;
use crate::data::queue::QueuedTrackInfo;
use crate::data::source::SourceId;

/// Information about a media item retrieved for HEOS servers or services.
///
/// This usually represents an input source where actual track/artist/album metadata can't be
/// determined, such as from an aux input.
#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct HeosMediaItem {
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
pub struct CommonMediaItem {
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
pub struct ArtistMediaItem {
    /// Name of the artist.
    pub name: String,
    /// ID representing the artist's "container" (all music by said artist).
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: CommonMediaItem,
}

/// Information about an album.
#[derive(Deserialize, Debug)]
pub struct AlbumMediaItem {
    /// Name of the album.
    pub name: String,
    /// Name of the album's artist.
    pub artist: String,
    /// ID representing the album.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: CommonMediaItem,
}

/// Information about a song.
#[derive(Deserialize, Debug)]
pub struct SongMediaItem {
    /// Name of the song.
    pub name: String,
    /// Name of the song's artist.
    pub artist: String,
    /// Name of the album the song is from.
    ///
    /// This may be blank if the song doesn't belong to an album.
    pub album: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: CommonMediaItem,
}

impl From<QueuedTrackInfo> for SongMediaItem {
    #[inline]
    fn from(value: QueuedTrackInfo) -> Self {
        SongMediaItem {
            name: value.song,
            artist: value.artist,
            album: value.album,
            common: CommonMediaItem {
                // All songs are playable
                playable: true,
                image_url: value.image_url,
                media_id: Some(value.media_id),
            }
        }
    }
}

/// Information about a generic music container.
#[derive(Deserialize, Debug)]
pub struct ContainerMediaItem {
    /// Name of the container.
    pub name: String,
    /// ID of the container.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: CommonMediaItem,
}

/// Information about a music station.
#[derive(Deserialize, Debug)]
pub struct StationMediaItem {
    /// Name of the station.
    pub name: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: CommonMediaItem,
}

/// Information about a music genre, used as a container.
#[derive(Deserialize, Debug)]
pub struct GenreMediaItem {
    /// Name of the genre.
    pub name: String,
    /// Common media data.
    #[serde(flatten)]
    pub common: CommonMediaItem,
}

/// A single item yielded from a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MediaItem {
    /// Items yielded by [HEOS services](SourceType::HeosService).
    HeosService(HeosMediaItem),
    /// Items yielded by [HEOS servers](SourceType::HeosServer).
    HeosServer(HeosMediaItem),
    /// A music artist.
    Artist(ArtistMediaItem),
    /// An album of tracks.
    Album(AlbumMediaItem),
    /// A single track.
    Song(SongMediaItem),
    /// A generic container of music.
    ///
    /// Could be e.g. a folder in a Windows Media Share.
    Container(ContainerMediaItem),
    /// A music station.
    Station(StationMediaItem),
    /// A genre of music being used as a container.
    Genre(GenreMediaItem),
}

impl From<QueuedTrackInfo> for MediaItem {
    #[inline]
    fn from(value: QueuedTrackInfo) -> Self {
        Self::Song(SongMediaItem::from(value))
    }
}

impl MediaItem {
    /// Get the image URL for this media item, if it exists.
    #[inline]
    pub fn image_url(&self) -> &Option<Url> {
        match self {
            Self::HeosService(heos_item) |
            Self::HeosServer(heos_item) => &heos_item.image_url,
            Self::Artist(artist) => &artist.common.image_url,
            Self::Album(album) => &album.common.image_url,
            Self::Song(song) => &song.common.image_url,
            Self::Container(container) => &container.common.image_url,
            Self::Station(station) => &station.common.image_url,
            Self::Genre(genre) => &genre.common.image_url,
        }
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