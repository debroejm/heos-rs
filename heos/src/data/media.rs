//! Data types representing various media items.
//!
//! These types provide information about e.g. songs, artist, albums, stations, etc.
//!
//! There are three categories of types in this module:
//!  * Base traits
//!  * Individual media item types
//!  * Composite media item enums
//!
//! Base traits are implemented by both individual and composite media item types, and can be used
//! as trait bounds for logic that wants to accept a broad category of types. The [MediaItemBase]
//! trait is implemented by _all_ media item types, and can be used as a bound to accept any media
//! item type. The [MediaContainerBase] trait is implemented by all media items that are considered
//! "containers", and have a `container_id`.
//!
//! Individual media item types are exactly what they sound like, and represent a specific type of
//! media item, such as an [Artist] or a [Song].
//!
//! Composite media item enums are enumerations that cover all possible individual types that match
//! that enum. These can be used instead of trait bounds for boundless logic. [MediaItem] covers
//! _all_ media item types, and [MediaContainer] covers all types that are considered "containers".
//!
//! In addition to [MediaItem] and [MediaContainer], there are also [MediaItemRef] and
//! [MediaContainerRef], which are enumerations that cover references to individual types.

use educe::Educe;
use qstring::QString;
use serde::Deserialize;
use std::fmt::Debug;
use strum::{EnumDiscriminants, IntoDiscriminant};
use url::Url;

use crate::command::CommandError;
use crate::data::impl_try_from_response_payload;
use crate::data::option::impl_has_options;
use crate::data::player::PlayerId;
use crate::data::queue::QueuedTrackInfo;
use crate::data::response::RawResponse;

/// Trait shared by all media item types.
///
/// This represents shared logic, and allows specifying trait bounds for media items.
pub trait MediaItemBase: Debug {
    /// Get the primary name of this media item.
    ///
    /// For example, for an Album media item, the primary name would be the album name.
    fn name(&self) -> &str;

    /// Get the name of the album this media item belongs to.
    ///
    /// If this media item does not belong to an album, yields `None`.
    ///
    /// Note that this yields `None` if the media item is an Album itself.
    fn album(&self) -> Option<&str> {
        None
    }

    /// Get the name of the artist of this media item.
    ///
    /// If this media item does not have an artist (e.g. if it's a Station or Genre), yields `None`.
    ///
    /// Note that this yields `None` if the media item is an Artist itself.
    fn artist(&self) -> Option<&str> {
        None
    }

    /// Get the image URL for this media item, if it exists.
    fn image_url(&self) -> Option<&Url>;

    /// Whether this media item is directly playable.
    fn playable(&self) -> bool;

    /// Get the Media ID of this media item.
    ///
    /// If this media item does not have a Media ID (e.g. if it's a Station or Genre, or a
    /// non-playable container), yields `None`.
    fn media_id(&self) -> Option<&str>;

    /// Convert this specific media item into the [MediaItem] enumeration.
    fn into_media_item(self) -> MediaItem where Self: Sized;

    /// Retrieve a reference to this media item as a [MediaItemRef].
    fn as_media_item_ref(&self) -> MediaItemRef<'_>;

    /// Attempt to convert this specific media item into the [MediaContainer] enumeration.
    ///
    /// If this specific media item isn't a container, yields itself as a `Result::Err()`.
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        Err(self)
    }

    /// Attempt to retrieve a reference to this media item as a [MediaContainerRef].
    ///
    /// If this specific media item isn't a container, yields `None`.
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        None
    }
}

/// Trait shared by all media item types that are considered "containers".
pub trait MediaContainerBase: MediaItemBase {
    /// Get the container ID of this media item.
    fn container_id(&self) -> &str;

    /// Convert this specific media container into the [MediaContainer] enumeration.
    fn into_media_container(self) -> MediaContainer where Self: Sized;

    /// Retrieve a reference to this media item as a [MediaContainerRef].
    fn as_media_container_ref(&self) -> MediaContainerRef<'_>;
}

/// Error that can occur when attempting to convert a [MediaItem]/[MediaContainer] to a specific
/// media item type via `TryFrom`.
#[derive(thiserror::Error, Debug)]
#[error("invalid MediaItem type; expected: {expected:?}, actual: {actual:?}")]
pub struct TryFromMediaItemError {
    /// The media item type that was attempted to convert to.
    pub expected: MediaItemType,
    /// The actual media item type of the [MediaItem]/[MediaContainer].
    pub actual: MediaItemType,
}

macro_rules! impl_media_item_conversions {
    ($name:ident) => {
        impl From<$name> for MediaItem {
            #[inline]
            fn from(value: $name) -> Self {
                value.into_media_item()
            }
        }

        impl TryFrom<MediaItem> for $name {
            type Error = TryFromMediaItemError;

            fn try_from(item: MediaItem) -> Result<Self, Self::Error> {
                if let MediaItem::$name(inner) = item {
                    Ok(inner)
                } else {
                    Err(TryFromMediaItemError {
                        expected: MediaItemType::$name,
                        actual: item.discriminant(),
                    })
                }
            }
        }
    };
}

macro_rules! impl_media_container_conversions {
    ($name:ident) => {
        impl TryFrom<$name> for MediaContainer {
            type Error = $name;

            #[inline]
            fn try_from(value: $name) -> Result<Self, Self::Error> {
                value.try_into_media_container()
            }
        }

        impl TryFrom<MediaContainer> for $name {
            type Error = TryFromMediaItemError;

            fn try_from(container: MediaContainer) -> Result<Self, Self::Error> {
                if let MediaContainer::$name(inner) = container {
                    Ok(inner)
                } else {
                    Err(TryFromMediaItemError {
                        expected: MediaItemType::$name,
                        actual: container.discriminant(),
                    })
                }
            }
        }
    };
}

/// Information about a media item retrieved for HEOS services.
///
/// This usually represents an input source where actual track/artist/album metadata can't be
/// determined, such as from an aux input.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct HeosService {
    /// Name of the service.
    pub name: String,
    /// URL to an image representing the service, if it exists.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// ID of the player the service is from.
    #[serde(rename = "sid")]
    pub src_player_id: PlayerId,
}

impl MediaItemBase for HeosService {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        true
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        None
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::HeosService(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::HeosService(self)
    }
}

impl_media_item_conversions!(HeosService);

/// Information about a media item retrieved for HEOS servers.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct HeosServer {
    /// Name of the server.
    pub name: String,
    /// URL to an image representing the server, if it exists.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// ID of the player the server is from.
    #[serde(rename = "sid")]
    pub src_player_id: PlayerId,
}

impl MediaItemBase for HeosServer {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        true
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        None
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::HeosServer(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::HeosServer(self)
    }
}

impl_media_item_conversions!(HeosServer);

/// Information about an artist.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct Artist {
    /// Name of the artist.
    pub name: String,
    /// ID representing the artist's "container" (all music by said artist).
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Is the artist directly playable?
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub playable: bool,
    /// URL to an image representing the artist, if it exists.
    ///
    /// If this is `None`, it may be able to be retrieved via the
    /// [GetAlbumMetadata](crate::command::browse::GetAlbumMetadata) command.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// Media ID of this artist, if it's playable.
    #[serde(rename = "mid")]
    pub media_id: Option<String>,
}

impl MediaItemBase for Artist {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        self.playable
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        self.media_id.as_deref()
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::Artist(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::Artist(self)
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        Ok(MediaContainer::Artist(self))
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        Some(MediaContainerRef::Artist(self))
    }
}

impl MediaContainerBase for Artist {
    #[inline]
    fn container_id(&self) -> &str {
        &self.container_id
    }

    #[inline]
    fn into_media_container(self) -> MediaContainer where Self: Sized {
        MediaContainer::Artist(self)
    }

    #[inline]
    fn as_media_container_ref(&self) -> MediaContainerRef<'_> {
        MediaContainerRef::Artist(self)
    }
}

impl_media_item_conversions!(Artist);
impl_media_container_conversions!(Artist);

/// Information about an album.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct Album {
    /// Name of the album.
    pub name: String,
    /// Name of the album's artist.
    pub artist: String,
    /// ID representing the album.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Is the album directly playable?
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub playable: bool,
    /// URL to an image representing the album, if it exists.
    ///
    /// If this is `None`, it may be able to be retrieved via the
    /// [GetAlbumMetadata](crate::command::browse::GetAlbumMetadata) command.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// Media ID of this album, if it's playable.
    #[serde(rename = "mid")]
    pub media_id: Option<String>,
}

impl MediaItemBase for Album {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn artist(&self) -> Option<&str> {
        Some(&self.artist)
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        self.playable
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        self.media_id.as_deref()
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::Album(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::Album(self)
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        Ok(MediaContainer::Album(self))
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        Some(MediaContainerRef::Album(self))
    }
}

impl MediaContainerBase for Album {
    #[inline]
    fn container_id(&self) -> &str {
        &self.container_id
    }

    #[inline]
    fn into_media_container(self) -> MediaContainer where Self: Sized {
        MediaContainer::Album(self)
    }

    #[inline]
    fn as_media_container_ref(&self) -> MediaContainerRef<'_> {
        MediaContainerRef::Album(self)
    }
}

impl_media_item_conversions!(Album);
impl_media_container_conversions!(Album);

/// Information about a song.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct Song {
    /// Name of the song.
    pub name: String,
    /// Name of the song's artist.
    pub artist: String,
    /// Name of the album the song is from.
    ///
    /// This may be blank if the song doesn't belong to an album.
    pub album: String,
    #[serde(rename = "playable", deserialize_with = "super::yes_no::deserialize_assert_yes")]
    _playable: bool,
    /// URL to an image representing the song, if it exists.
    ///
    /// If this is `None`, it may be able to be retrieved via the
    /// [GetAlbumMetadata](crate::command::browse::GetAlbumMetadata) command.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// ID representing this media item.
    #[serde(rename = "mid")]
    pub media_id: String,
}

impl MediaItemBase for Song {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn artist(&self) -> Option<&str> {
        Some(&self.artist)
    }

    #[inline]
    fn album(&self) -> Option<&str> {
        Some(&self.album)
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        true
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        Some(&self.media_id)
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::Song(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::Song(self)
    }
}

impl From<QueuedTrackInfo> for Song {
    #[inline]
    fn from(value: QueuedTrackInfo) -> Self {
        Song {
            name: value.song,
            artist: value.artist,
            album: value.album,
            _playable: true,
            image_url: value.image_url,
            media_id: value.media_id,
        }
    }
}

impl_media_item_conversions!(Song);

/// Information about a generic music container.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct Container {
    /// Name of the container.
    pub name: String,
    /// ID of the container.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// Is the container directly playable?
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub playable: bool,
    /// URL to an image representing the container, if it exists.
    ///
    /// If this is `None`, it may be able to be retrieved via the
    /// [GetAlbumMetadata](crate::command::browse::GetAlbumMetadata) command.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// Media ID of this container, if it's playable.
    #[serde(rename = "mid")]
    pub media_id: Option<String>,
}

impl MediaItemBase for Container {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        self.playable
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        self.media_id.as_deref()
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::Container(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::Container(self)
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        Ok(MediaContainer::Container(self))
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        Some(MediaContainerRef::Container(self))
    }
}

impl MediaContainerBase for Container {
    #[inline]
    fn container_id(&self) -> &str {
        &self.container_id
    }

    #[inline]
    fn into_media_container(self) -> MediaContainer where Self: Sized {
        MediaContainer::Container(self)
    }

    #[inline]
    fn as_media_container_ref(&self) -> MediaContainerRef<'_> {
        MediaContainerRef::Container(self)
    }
}

impl_media_item_conversions!(Container);
impl_media_container_conversions!(Container);

/// Information about a music station.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct Station {
    /// Name of the station.
    pub name: String,
    #[serde(rename = "playable", deserialize_with = "super::yes_no::deserialize_assert_yes")]
    _playable: bool,
    /// URL to an image representing the station, if it exists.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// Media ID of this station.
    #[serde(rename = "mid")]
    pub media_id: String,
}

impl MediaItemBase for Station {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        true
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        Some(&self.media_id)
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::Station(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::Station(self)
    }
}

impl_media_item_conversions!(Station);

/// Information about a music genre, used as a container.
#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct Genre {
    /// Name of the genre.
    pub name: String,
    #[serde(rename = "playable", deserialize_with = "super::yes_no::deserialize_assert_yes")]
    _playable: bool,
    /// URL to an image representing the genre, if it exists.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    /// Media ID of this genre.
    #[serde(rename = "mid")]
    pub media_id: String,
}

impl MediaItemBase for Genre {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn image_url(&self) -> Option<&Url> {
        self.image_url.as_ref()
    }

    #[inline]
    fn playable(&self) -> bool {
        true
    }

    #[inline]
    fn media_id(&self) -> Option<&str> {
        Some(&self.media_id)
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        MediaItem::Genre(self)
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        MediaItemRef::Genre(self)
    }
}

impl_media_item_conversions!(Genre);

macro_rules! delegate_media_item {
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(&self) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v fn $fn_name(&self) -> $ret_type {
            match self {
                Self::HeosService(heos_service) => heos_service.$fn_name(),
                Self::HeosServer(heos_server) => heos_server.$fn_name(),
                Self::Artist(artist) => artist.$fn_name(),
                Self::Album(album) => album.$fn_name(),
                Self::Song(song) => song.$fn_name(),
                Self::Container(container) => container.$fn_name(),
                Self::Station(station) => station.$fn_name(),
                Self::Genre(genre) => genre.$fn_name(),
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(self) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v fn $fn_name(self) -> $ret_type where Self: Sized {
            match self {
                Self::HeosService(heos_service) => heos_service.$fn_name(),
                Self::HeosServer(heos_server) => heos_server.$fn_name(),
                Self::Artist(artist) => artist.$fn_name(),
                Self::Album(album) => album.$fn_name(),
                Self::Song(song) => song.$fn_name(),
                Self::Container(container) => container.$fn_name(),
                Self::Station(station) => station.$fn_name(),
                Self::Genre(genre) => genre.$fn_name(),
            }
        }
    };
}

/// A single item yielded from a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command.
#[derive(Deserialize, EnumDiscriminants, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum_discriminants(name(MediaItemType))]
pub enum MediaItem {
    /// Items yielded by [HEOS services](SourceType::HeosService).
    HeosService(HeosService),
    /// Items yielded by [HEOS servers](SourceType::HeosServer).
    HeosServer(HeosServer),
    /// A music artist.
    Artist(Artist),
    /// An album of tracks.
    Album(Album),
    /// A single track.
    Song(Song),
    /// A generic container of music.
    ///
    /// Could be e.g. a folder in a Windows Media Share.
    Container(Container),
    /// A music station.
    Station(Station),
    /// A genre of music being used as a container.
    Genre(Genre),
}
impl_has_options!(Vec<MediaItem>, "browse");
impl_try_from_response_payload!(Vec<MediaItem>);

impl From<QueuedTrackInfo> for MediaItem {
    #[inline]
    fn from(value: QueuedTrackInfo) -> Self {
        Self::Song(Song::from(value))
    }
}

impl MediaItemBase for MediaItem {
    delegate_media_item! {
        #[inline]
        fn name(&self) -> &str;
    }

    delegate_media_item! {
        #[inline]
        fn image_url(&self) -> Option<&Url>;
    }

    delegate_media_item! {
        #[inline]
        fn playable(&self) -> bool;
    }

    delegate_media_item! {
        #[inline]
        fn media_id(&self) -> Option<&str>;
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        self
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        match self {
            Self::HeosService(heos_service) => MediaItemRef::HeosService(heos_service),
            Self::HeosServer(heos_server) => MediaItemRef::HeosServer(heos_server),
            Self::Artist(artist) => MediaItemRef::Artist(artist),
            Self::Album(album) => MediaItemRef::Album(album),
            Self::Song(song) => MediaItemRef::Song(song),
            Self::Container(container) => MediaItemRef::Container(container),
            Self::Station(station) => MediaItemRef::Station(station),
            Self::Genre(genre) => MediaItemRef::Genre(genre),
        }
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        match self {
            Self::Artist(artist) => Ok(MediaContainer::Artist(artist)),
            Self::Album(album) => Ok(MediaContainer::Album(album)),
            Self::Container(container) => Ok(MediaContainer::Container(container)),
            other => Err(other),
        }
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        match self {
            Self::Artist(artist) => Some(MediaContainerRef::Artist(artist)),
            Self::Album(album) => Some(MediaContainerRef::Album(album)),
            Self::Container(container) => Some(MediaContainerRef::Container(container)),
            _ => None,
        }
    }
}

/// A single item yielded from a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command.
///
/// This type is a reference to a media item. It is to [MediaItem] what `&str` is to `String`.
#[derive(Debug, Clone)]
pub enum MediaItemRef<'a> {
    /// Items yielded by [HEOS services](SourceType::HeosService).
    HeosService(&'a HeosService),
    /// Items yielded by [HEOS servers](SourceType::HeosServer).
    HeosServer(&'a HeosServer),
    /// A music artist.
    Artist(&'a Artist),
    /// An album of tracks.
    Album(&'a Album),
    /// A single track.
    Song(&'a Song),
    /// A generic container of music.
    ///
    /// Could be e.g. a folder in a Windows Media Share.
    Container(&'a Container),
    /// A music station.
    Station(&'a Station),
    /// A genre of music being used as a container.
    Genre(&'a Genre),
}

impl<'a> IntoDiscriminant for MediaItemRef<'a> {
    type Discriminant = MediaItemType;

    fn discriminant(&self) -> Self::Discriminant {
        match self {
            Self::HeosService(_) => MediaItemType::HeosService,
            Self::HeosServer(_) => MediaItemType::HeosServer,
            Self::Artist(_) => MediaItemType::Artist,
            Self::Album(_) => MediaItemType::Album,
            Self::Song(_) => MediaItemType::Song,
            Self::Container(_) => MediaItemType::Container,
            Self::Station(_) => MediaItemType::Station,
            Self::Genre(_) => MediaItemType::Genre,
        }
    }
}

impl<'a> MediaItemBase for MediaItemRef<'a> {
    delegate_media_item! {
        #[inline]
        fn name(&self) -> &str;
    }

    delegate_media_item! {
        #[inline]
        fn image_url(&self) -> Option<&Url>;
    }

    delegate_media_item! {
        #[inline]
        fn playable(&self) -> bool;
    }

    delegate_media_item! {
        #[inline]
        fn media_id(&self) -> Option<&str>;
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        match self {
            Self::HeosService(heos_service) => MediaItem::HeosService(heos_service.clone()),
            Self::HeosServer(heos_server) => MediaItem::HeosServer(heos_server.clone()),
            Self::Artist(artist) => MediaItem::Artist(artist.clone()),
            Self::Album(album) => MediaItem::Album(album.clone()),
            Self::Song(song) => MediaItem::Song(song.clone()),
            Self::Container(container) => MediaItem::Container(container.clone()),
            Self::Station(station) => MediaItem::Station(station.clone()),
            Self::Genre(genre) => MediaItem::Genre(genre.clone()),
        }
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        self.clone()
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        match self {
            Self::Artist(artist) => Ok(MediaContainer::Artist(artist.clone())),
            Self::Album(album) => Ok(MediaContainer::Album(album.clone())),
            Self::Container(container) => Ok(MediaContainer::Container(container.clone())),
            other => Err(other),
        }
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        match self {
            Self::Artist(artist) => Some(MediaContainerRef::Artist(artist)),
            Self::Album(album) => Some(MediaContainerRef::Album(album)),
            Self::Container(container) => Some(MediaContainerRef::Container(container)),
            _ => None,
        }
    }
}

impl<'a> From<MediaItemRef<'a>> for MediaItem {
    #[inline]
    fn from(item_ref: MediaItemRef<'a>) -> Self {
        item_ref.into_media_item()
    }
}

impl<'a> TryFrom<MediaItemRef<'a>> for MediaContainer {
    type Error = MediaItemRef<'a>;

    #[inline]
    fn try_from(value: MediaItemRef<'a>) -> Result<Self, Self::Error> {
        value.try_into_media_container()
    }
}

macro_rules! delegate_media_container {
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(&self) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Artist(artist) => artist.$fn_name(),
                Self::Album(album) => album.$fn_name(),
                Self::Container(container) => container.$fn_name(),
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(self) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v fn $fn_name(self) -> $ret_type where Self: Sized {
            match self {
                Self::Artist(artist) => artist.$fn_name(),
                Self::Album(album) => album.$fn_name(),
                Self::Container(container) => container.$fn_name(),
            }
        }
    };
}

/// A single item yielded from a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command, that is a container.
///
/// This type isn't yielded directly from commands, but can be retrieved via
/// [`MediaContainer::try_from()`] from a [MediaItem].
#[derive(Debug, Clone)]
pub enum MediaContainer {
    /// A music artist.
    Artist(Artist),
    /// An album of tracks.
    Album(Album),
    /// A generic container of music.
    ///
    /// Could be e.g. a folder in a Windows Media Share.
    Container(Container),
}

impl IntoDiscriminant for MediaContainer {
    type Discriminant = MediaItemType;

    fn discriminant(&self) -> Self::Discriminant {
        match self {
            Self::Artist(_) => Self::Discriminant::Artist,
            Self::Album(_) => Self::Discriminant::Album,
            Self::Container(_) => Self::Discriminant::Container,
        }
    }
}

impl MediaItemBase for MediaContainer {
    delegate_media_container! {
        #[inline]
        fn name(&self) -> &str;
    }

    delegate_media_container! {
        #[inline]
        fn image_url(&self) -> Option<&Url>;
    }

    delegate_media_container! {
        #[inline]
        fn playable(&self) -> bool;
    }

    delegate_media_container! {
        #[inline]
        fn media_id(&self) -> Option<&str>;
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        match self {
            Self::Artist(artist) => MediaItem::Artist(artist),
            Self::Album(album) => MediaItem::Album(album),
            Self::Container(container) => MediaItem::Container(container),
        }
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        match self {
            Self::Artist(artist) => MediaItemRef::Artist(artist),
            Self::Album(album) => MediaItemRef::Album(album),
            Self::Container(container) => MediaItemRef::Container(container),
        }
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        Ok(self)
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        Some(match self {
            Self::Artist(artist) => MediaContainerRef::Artist(artist),
            Self::Album(album) => MediaContainerRef::Album(album),
            Self::Container(container) => MediaContainerRef::Container(container),
        })
    }
}

impl MediaContainerBase for MediaContainer {
    delegate_media_container! {
        #[inline]
        fn container_id(&self) -> &str;
    }

    #[inline]
    fn into_media_container(self) -> MediaContainer where Self: Sized {
        self
    }

    #[inline]
    fn as_media_container_ref(&self) -> MediaContainerRef<'_> {
        match self {
            Self::Artist(artist) => MediaContainerRef::Artist(artist),
            Self::Album(album) => MediaContainerRef::Album(album),
            Self::Container(container) => MediaContainerRef::Container(container),
        }
    }
}

impl From<MediaContainer> for MediaItem {
    fn from(container: MediaContainer) -> Self {
        container.into_media_item()
    }
}

impl TryFrom<MediaItem> for MediaContainer {
    type Error = MediaItem;

    fn try_from(item: MediaItem) -> Result<Self, Self::Error> {
        item.try_into_media_container()
    }
}

/// A single item yielded from a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command, that is a container.
///
/// This type is a reference to a container type. It is to [MediaContainer] what `&str` is to
/// `String`.
#[derive(Debug, Clone, Copy)]
pub enum MediaContainerRef<'a> {
    /// A music artist.
    Artist(&'a Artist),
    /// An album of tracks.
    Album(&'a Album),
    /// A generic container of music.
    ///
    /// Could be e.g. a folder in a Windows Media Share.
    Container(&'a Container),
}

impl<'a> IntoDiscriminant for MediaContainerRef<'a> {
    type Discriminant = MediaItemType;

    fn discriminant(&self) -> Self::Discriminant {
        match self {
            Self::Artist(_) => Self::Discriminant::Artist,
            Self::Album(_) => Self::Discriminant::Album,
            Self::Container(_) => Self::Discriminant::Container,
        }
    }
}

impl<'a> MediaItemBase for MediaContainerRef<'a> {
    delegate_media_container! {
        #[inline]
        fn name(&self) -> &str;
    }

    delegate_media_container! {
        #[inline]
        fn image_url(&self) -> Option<&Url>;
    }

    delegate_media_container! {
        #[inline]
        fn playable(&self) -> bool;
    }

    delegate_media_container! {
        #[inline]
        fn media_id(&self) -> Option<&str>;
    }

    #[inline]
    fn into_media_item(self) -> MediaItem where Self: Sized {
        match self {
            Self::Artist(artist) => MediaItem::Artist(artist.clone()),
            Self::Album(album) => MediaItem::Album(album.clone()),
            Self::Container(container) => MediaItem::Container(container.clone()),
        }
    }

    #[inline]
    fn as_media_item_ref(&self) -> MediaItemRef<'_> {
        match self {
            Self::Artist(artist) => MediaItemRef::Artist(artist),
            Self::Album(album) => MediaItemRef::Album(album),
            Self::Container(container) => MediaItemRef::Container(container),
        }
    }

    #[inline]
    fn try_into_media_container(self) -> Result<MediaContainer, Self> where Self: Sized {
        Ok(match self {
            Self::Artist(artist) => MediaContainer::Artist(artist.clone()),
            Self::Album(album) => MediaContainer::Album(album.clone()),
            Self::Container(container) => MediaContainer::Container(container.clone()),
        })
    }

    #[inline]
    fn try_as_media_container_ref(&self) -> Option<MediaContainerRef<'_>> {
        Some(self.clone())
    }
}

impl<'a> MediaContainerBase for MediaContainerRef<'a> {
    delegate_media_container! {
        #[inline]
        fn container_id(&self) -> &str;
    }

    #[inline]
    fn into_media_container(self) -> MediaContainer where Self: Sized {
        match self {
            Self::Artist(artist) => MediaContainer::Artist(artist.clone()),
            Self::Album(album) => MediaContainer::Album(album.clone()),
            Self::Container(container) => MediaContainer::Container(container.clone()),
        }
    }

    #[inline]
    fn as_media_container_ref(&self) -> MediaContainerRef<'_> {
        self.clone()
    }
}

impl<'a> From<MediaContainerRef<'a>> for MediaItem {
    #[inline]
    fn from(container_ref: MediaContainerRef) -> Self {
        container_ref.into_media_item()
    }
}

impl<'a> From<MediaContainerRef<'a>> for MediaContainer {
    #[inline]
    fn from(container_ref: MediaContainerRef<'a>) -> Self {
        container_ref.into_media_container()
    }
}

/// Results of using a [Browse](crate::command::browse::Browse) or
/// [Search](crate::command::browse::Search) command.
#[derive(Debug, Clone)]
pub struct MediaItemsResponse {
    /// How many total items are available in the container being browsed/searched.
    ///
    /// If this is larger than the size of `items`, the command needs to be repeated until all
    /// results are retrieved.
    pub count: usize,
    /// Collection of search results.
    ///
    /// This will not be larger than the maximum range specified in the
    /// [Browse](crate::command::browse::Browse) or [Search](crate::command::browse::Search)
    /// command, or the default maximum if a range is not specified.
    pub items: Vec<MediaItem>,
}
impl_has_options!(MediaItemsResponse, "browse");

impl TryFrom<RawResponse> for MediaItemsResponse {
    type Error = CommandError;

    fn try_from(response: RawResponse) -> Result<Self, Self::Error> {
        let qs = QString::from(response.heos.message.as_str());
        let count: usize = qs.get("count")
            .ok_or(CommandError::response_missing_field("message.count"))?
            .parse()
            .map_err(|err| CommandError::MalformedResponse(format!(
                "could not parse 'count': {err:?}"
            )))?;
        let items = Vec::<MediaItem>::try_from(response)?;
        Ok(Self {
            count,
            items,
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