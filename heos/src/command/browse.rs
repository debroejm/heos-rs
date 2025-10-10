use std::ops::RangeInclusive;
use serde::Serialize;
use url::Url;

use crate::command::impl_command;
use crate::data::maybe_range;
use crate::data::option::*;
use crate::data::player::*;
use crate::data::source::*;

#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetSources {}
impl_command!(GetSources, "browse", "get_music_sources", Vec<SourceInfo>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetSourceInfo {
    #[serde(rename = "sid")]
    source_id: SourceId,
}
impl_command!(GetSourceInfo, "browse", "get_source_info", SourceInfo);

#[derive(Serialize, Debug, Clone)]
pub struct Browse {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
    #[serde(serialize_with = "maybe_range::serialize")]
    pub range: Option<RangeInclusive<usize>>,
}
impl_command!(Browse, "browse", "browse", WithOptions<Vec<SourceItem>>);

#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetSearchCriteria {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
}
impl_command!(GetSearchCriteria, "browse", "get_search_criteria", Vec<SearchCriteria>);

#[derive(Serialize, Debug, Clone)]
pub struct Search {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    pub search: String,
    #[serde(rename = "scid")]
    pub criteria: String,
    #[serde(serialize_with = "maybe_range::serialize")]
    pub range: Option<RangeInclusive<usize>>,
}
impl_command!(Search, "browse", "search", WithOptions<SearchResults>);

#[derive(Serialize, Debug, Clone)]
pub struct PlayStation {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
    #[serde(rename = "mid")]
    pub media_id: String,
    pub name: String,
}
impl_command!(PlayStation, "browse", "play_stream", ());

#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayPreset {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub preset: usize,
}
impl_command!(PlayPreset, "browse", "play_preset", ());

#[derive(Serialize, Debug, Clone)]
pub struct PlayInputSource {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "spid")]
    pub src_player_id: Option<PlayerId>,
    pub input: String,
}
impl_command!(PlayInputSource, "browse", "play_input", ());

#[derive(Serialize, Debug, Clone)]
pub struct PlayUrl {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    pub url: Url,
}
impl_command!(PlayUrl, "browse", "play_stream", ());

#[derive(Serialize, Debug, Clone)]
pub struct AddToQueue {
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
    #[serde(rename = "mid")]
    pub media_id: String,
    #[serde(rename = "aid")]
    pub add_to_queue_type: AddToQueueType,
}
impl_command!(AddToQueue, "browse", "add_to_queue", ());

#[derive(Serialize, Debug, Clone)]
pub struct RenamePlaylist {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(rename = "cid")]
    pub container_id: String,
    pub name: String,
}
impl_command!(RenamePlaylist, "browse", "rename_playlist", ());

#[derive(Serialize, Debug, Clone)]
pub struct DeletePlaylist {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(rename = "cid")]
    pub container_id: String,
}
impl_command!(DeletePlaylist, "browse", "delete_playlist", ());

#[derive(Serialize, Debug, Clone)]
pub struct GetAlbumMetadata {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(rename = "cid")]
    pub container_id: String,
}
impl_command!(GetAlbumMetadata, "browse", "retrieve_metadata", Vec<AlbumMetadata>);

#[derive(Serialize, Debug, Clone)]
pub struct SetServiceOption {
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    #[serde(flatten)]
    pub option: ServiceOption,
}
impl_command!(SetServiceOption, "browse", "set_service_option", ());