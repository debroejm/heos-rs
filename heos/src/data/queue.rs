//! Data types representing the queue of tracks to play.

use educe::Educe;
use serde::{Deserialize, Serialize};
use url::Url;

use super::*;
use crate::data::option::impl_has_options;
use crate::data::source::SourceId;

id_type! {
    /// ID representing a specific slot in the queue.
    pub struct QueueId(pub u64);
}

/// Information about a specific track in the queue.
#[derive(Deserialize, Serialize, Educe, Clone)]
#[educe(Debug)]
pub struct QueuedTrackInfo {
    /// Name of the song.
    pub song: String,
    /// Name of the album the song is from.
    ///
    /// This may be blank.
    pub album: String,
    /// Name of the song's artist.
    pub artist: String,
    /// URL to the album art for the track.
    ///
    /// If this is `None`, the album art can be retrieved via the
    /// [GetAlbumMetadata](crate::command::browse::GetAlbumMetadata) command.
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(with = "super::maybe_url")]
    pub image_url: Option<Url>,
    /// ID representing this track across all media.
    #[serde(rename = "mid")]
    pub media_id: String,
    /// ID of the queue slot this track is in.
    #[serde(rename = "qid")]
    pub queue_id: QueueId,
    /// ID of the album this track is from, if it belongs to an album.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
}
impl_try_from_response_payload!(QueuedTrackInfo);
impl_try_from_response_payload!(Vec<QueuedTrackInfo>);

/// Information about the currently playing media.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum NowPlayingInfo {
    /// The currently playing media is a song.
    #[serde(rename = "song")]
    Song {
        /// Full information on the currently playing track.
        #[serde(flatten)]
        info: QueuedTrackInfo,
        /// ID of the source the song is from.
        #[serde(rename = "sid")]
        source_id: SourceId,
    },
    /// The currently playing media is a station.
    #[serde(rename = "station")]
    Station {
        /// Full information on the currently playing track.
        #[serde(flatten)]
        info: QueuedTrackInfo,
        /// Name of the currently playing station.
        station: String,
        /// ID of the source the station is from.
        #[serde(rename = "sid")]
        source_id: SourceId,
    }
}
impl_has_options!(NowPlayingInfo, "play");
impl_try_from_response_payload!(NowPlayingInfo);