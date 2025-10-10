use educe::Educe;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::EnumString;
use url::Url;
use crate::command::CommandError;
use crate::data::option::impl_has_options;
use crate::data::response::RawResponse;
use super::*;

id_type!(SourceId);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "snake_case")]
pub enum SourceType {
    MusicService,
    HeosService,
    HeosServer,
    DlnaServer,
}
impl_enum_string_conversions!(SourceType);

#[derive(Serialize, Deserialize, EnumString, strum::Display, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(into = "String", try_from = "String")]
#[strum(serialize_all = "lowercase")]
pub enum SourceAvailable {
    True,
    False,
}
impl_enum_string_conversions!(SourceAvailable);

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct SourceInfo {
    pub name: String,
    #[educe(Debug(method(std::fmt::Display::fmt)))]
    pub image_url: Url,
    #[serde(rename = "type")]
    pub source_type: SourceType,
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    pub available: SourceAvailable,
    pub service_username: Option<String>,
}
impl_try_from_response_payload!(SourceInfo);
impl_try_from_response_payload!(Vec<SourceInfo>);

#[derive(Deserialize, Debug)]
pub struct SourceItemData {
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub container: bool,
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub playable: bool,
}

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct HeosSourceItem {
    pub name: String,
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    #[serde(rename = "sid")]
    pub source_id: SourceId,
}

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct MediaSourceItem {
    #[serde(deserialize_with = "super::yes_no::deserialize")]
    pub playable: bool,
    #[educe(Debug(method(super::maybe_url::fmt)))]
    #[serde(deserialize_with = "super::maybe_url::deserialize")]
    pub image_url: Option<Url>,
    #[serde(rename = "mid")]
    pub media_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ArtistSourceItem {
    pub name: String,
    #[serde(rename = "cid")]
    pub container_id: String,
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

#[derive(Deserialize, Debug)]
pub struct AlbumSourceItem {
    pub name: String,
    pub artist: String,
    #[serde(rename = "cid")]
    pub container_id: String,
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

#[derive(Deserialize, Debug)]
pub struct SongSourceItem {
    pub name: String,
    pub artist: String,
    pub album: String,
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

#[derive(Deserialize, Debug)]
pub struct ContainerSourceItem {
    pub name: String,
    #[serde(rename = "cid")]
    pub container_id: String,
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

#[derive(Deserialize, Debug)]
pub struct StationSourceItem {
    pub name: String,
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

#[derive(Deserialize, Debug)]
pub struct GenreSourceItem {
    pub name: String,
    #[serde(flatten)]
    pub common: MediaSourceItem,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SourceItem {
    HeosService(HeosSourceItem),
    HeosServer(HeosSourceItem),
    Artist(ArtistSourceItem),
    Album(AlbumSourceItem),
    Song(SongSourceItem),
    Container(ContainerSourceItem),
    Station(StationSourceItem),
    Genre(GenreSourceItem),
}
impl_has_options!(Vec<SourceItem>, "browse");
impl_try_from_response_payload!(Vec<SourceItem>);

#[derive(Deserialize, Debug)]
pub struct SearchCriteria {
    pub name: String,
    #[serde(rename = "scid")]
    pub criteria: String,
    #[serde(deserialize_with = "super::maybe_yes_no::deserialize")]
    pub wildcard: Option<bool>,
    #[serde(deserialize_with = "super::maybe_yes_no::deserialize")]
    pub playable: Option<bool>,
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
}
impl_try_from_response_payload!(Vec<SearchCriteria>);

#[derive(Debug)]
pub struct SearchResults {
    pub count: usize,
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

#[derive(Deserialize, Educe)]
#[educe(Debug)]
pub struct AlbumImageMetadata {
    #[educe(Debug(method(std::fmt::Display::fmt)))]
    pub image_url: Url,
    pub width: usize,
}

#[derive(Deserialize, Debug)]
pub struct AlbumMetadata {
    pub album_id: String,
    pub images: Vec<AlbumImageMetadata>,
}
impl_try_from_response_payload!(Vec<AlbumMetadata>);