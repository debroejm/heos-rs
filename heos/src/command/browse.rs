/// Commands to query music sources and browse/search for music.
///
/// See [crate::data::source] for more information on music sources.

use serde::Serialize;
use std::ops::RangeInclusive;
use url::Url;

use crate::command::impl_command;
use crate::data::maybe_range;
use crate::data::option::*;
use crate::data::player::*;
use crate::data::source::*;

/// Retrieve all music sources.
#[derive(Serialize, Debug, Default, Clone, Copy)]
pub struct GetSources {}
impl_command!(GetSources, "browse", "get_music_sources", Vec<SourceInfo>);

/// Retrieve a specific music source by [ID](SourceId).
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetSourceInfo {
    /// ID of the source to retrieve.
    #[serde(rename = "sid")]
    source_id: SourceId,
}
impl_command!(GetSourceInfo, "browse", "get_source_info", SourceInfo);

/// Browse a source's music.
///
/// If no `container_id` is provided, this will browse top-level results. If a `container_id` is
/// provided, this will instead browse the contents of said container.
///
/// For local music servers and auxiliary inputs, a top-level browse yields e.g. the top-level
/// visible folder or equivalent.
///
/// For music services (e.g. Pandora), the result of a top-level is dependent on the music source.
/// Some sources will provide a view of most popular music, while others (such as Amazon Music) may
/// allow you to browse your own library of music.
#[derive(Serialize, Debug, Clone)]
pub struct Browse {
    /// ID of the source to browse.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// ID of the container to browse.
    ///
    /// This may be omitted (`None`), in which case a predefined "top-level" browse result is
    /// retrieved.
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
    /// Range of results to retrieve.
    ///
    /// Zero-indexed.
    ///
    /// This may be omitted (`None`), and defaults to all results up to a maximum of 50 or 100
    /// records per response, depending on the music source.
    // TODO: Figure out how HEOS paginated responses work
    #[serde(serialize_with = "maybe_range::serialize")]
    pub range: Option<RangeInclusive<usize>>,
}
impl_command!(Browse, "browse", "browse", WithOptions<Vec<SourceItem>>);

/// Retrieve all valid types of search criteria for a source.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct GetSearchCriteria {
    /// ID of the source to retrieve.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
}
impl_command!(GetSearchCriteria, "browse", "get_search_criteria", Vec<SearchCriteria>);

/// Search a source for songs.
#[derive(Serialize, Debug, Clone)]
pub struct Search {
    /// ID of the source to search.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// String to search by.
    pub search: String,
    /// Search criteria ID yielded by [GetSearchCriteria].
    #[serde(rename = "scid")]
    pub criteria: String,
    /// Range of results to retrieve.
    ///
    /// Zero-indexed.
    ///
    /// This may be omitted (`None`), and defaults to all results up to a maximum of 50 or 100
    /// records per response, depending on the music source.
    // TODO: Figure out how HEOS paginated responses work
    #[serde(serialize_with = "maybe_range::serialize")]
    pub range: Option<RangeInclusive<usize>>,
}
impl_command!(Search, "browse", "search", WithOptions<SearchResults>);

/// Play a music "station".
///
/// Exactly what constitutes a "station" may differ depending on the music source.
#[derive(Serialize, Debug, Clone)]
pub struct PlayStation {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of the source to play from.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// ID of the container for the "station".
    ///
    /// This may be omitted (`None`) if there is no relevant container ID, e.g. when playing a
    /// station obtained via the [Search] command.
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
    /// Media ID retrieved via either [Browse] or [Search].
    #[serde(rename = "mid")]
    pub media_id: String,
    /// Name of the station retrieved via e.g. [Browse].
    pub name: String,
}
impl_command!(PlayStation, "browse", "play_stream", ());

/// Play a preset station from HEOS Favorites.
#[derive(Serialize, Debug, Clone, Copy)]
pub struct PlayPreset {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// Station offset in HEOS favorites.
    ///
    /// One-indexed.
    pub preset: usize,
}
impl_command!(PlayPreset, "browse", "play_preset", ());

/// Play from a player's input source.
///
/// Can also be used to play one player's input source on another player.
#[derive(Serialize, Debug, Clone)]
pub struct PlayInputSource {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of the player that has the input source to play from, if different from `player_id`.
    ///
    /// If playing an input source from the same player, this should be `None`.
    #[serde(rename = "spid")]
    pub src_player_id: Option<PlayerId>,
    /// Name of the input source to play from.
    ///
    /// Generally follows the format of "inputs/<name>" (e.g. "inputs/aux1").
    pub input: String,
}
impl_command!(PlayInputSource, "browse", "play_input", ());

/// Play a remote stream.
#[derive(Serialize, Debug, Clone)]
pub struct PlayUrl {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// Absolute URL to a remote playable stream.
    pub url: Url,
}
impl_command!(PlayUrl, "browse", "play_stream", ());

/// Add a music track to the queue.
#[derive(Serialize, Debug, Clone)]
pub struct AddToQueue {
    /// ID of the player to play on.
    #[serde(rename = "pid")]
    pub player_id: PlayerId,
    /// ID of the source that contains the music track.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// ID of the container that contains the music track, if relevant.
    ///
    /// This may be omitted (`None`) if there is no relevant container ID.
    #[serde(rename = "cid")]
    pub container_id: Option<String>,
    /// Media ID of the music track.
    #[serde(rename = "mid")]
    pub media_id: String,
    /// The method used to add the music track to the queue.
    #[serde(rename = "aid")]
    pub add_to_queue_type: AddToQueueType,
}
impl_command!(AddToQueue, "browse", "add_to_queue", ());

/// Rename a playlist.
#[derive(Serialize, Debug, Clone)]
pub struct RenamePlaylist {
    /// ID of the source that contains the playlist.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// ID of the playlist.
    #[serde(rename = "cid")]
    pub container_id: String,
    /// The new name to give the playlist.
    pub name: String,
}
impl_command!(RenamePlaylist, "browse", "rename_playlist", ());

/// Delete a playlist.
#[derive(Serialize, Debug, Clone)]
pub struct DeletePlaylist {
    /// ID of the source that contains the playlist.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// ID of the playlist.
    #[serde(rename = "cid")]
    pub container_id: String,
}
impl_command!(DeletePlaylist, "browse", "delete_playlist", ());

/// Retrieve album metadata for a given album ID.
///
/// Some music sources don't provide album art URL while browsing/searching/playing/etc, so if any
/// command yields a blank URL, this can be used to retrieve the relevant album metadata.
#[derive(Serialize, Debug, Clone)]
pub struct GetAlbumMetadata {
    /// ID of the source that contains the album.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// ID of the album.
    #[serde(rename = "cid")]
    pub container_id: String,
}
impl_command!(GetAlbumMetadata, "browse", "retrieve_metadata", Vec<AlbumMetadata>);

/// Set service-specific options.
///
/// If any command that yields options does so, this command can be used to set the options that are
/// described.
///
/// Commands that can yield options:
///  * [GetNowPlayingMedia](crate::command::player::GetNowPlayingMedia)
///  * [Browse]
#[derive(Serialize, Debug, Clone)]
pub struct SetServiceOption {
    /// Music service that the option applies to.
    #[serde(rename = "sid")]
    pub source_id: SourceId,
    /// Specific option to set.
    #[serde(flatten)]
    pub option: ServiceOption,
}
impl_command!(SetServiceOption, "browse", "set_service_option", ());