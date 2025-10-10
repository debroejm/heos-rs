use educe::Educe;
use serde::Deserialize;
use url::Url;
use crate::data::option::impl_has_options;
use super::*;
use crate::data::source::SourceId;

id_type!(QueueId);

#[derive(Deserialize, Educe, Clone)]
#[educe(Debug)]
pub struct SongInfo {
    pub song: String,
    pub album: String,
    pub artist: String,
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    // TODO: Rename all other usages of mid/qid/cid
    #[serde(rename = "mid")]
    pub media_id: String,
    #[serde(rename = "qid")]
    pub queue_id: QueueId,
    pub album_id: Option<String>,
}
impl_try_from_response_payload!(SongInfo);
impl_try_from_response_payload!(Vec<SongInfo>);

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum NowPlayingInfo {
    #[serde(rename = "song")]
    Song {
        #[serde(flatten)]
        info: SongInfo,
        #[serde(rename = "sid")]
        source_id: SourceId,
    },
    #[serde(rename = "station")]
    Station {
        #[serde(flatten)]
        info: SongInfo,
        station: String,
        #[serde(rename = "sid")]
        source_id: SourceId,
    }
}
impl_has_options!(NowPlayingInfo, "play");
impl_try_from_response_payload!(NowPlayingInfo);