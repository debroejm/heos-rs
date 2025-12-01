//! Mock HEOS system for testing purposes.
//!
//! This module contains a mock implementation of a HEOS system, intended for writing tests against.
//! The mock system implements as much of a HEOS system as it reasonably can, and serves as a good
//! representation of how a HEOS system is expected to act. Much of the preset mock data and the
//! implemented behaviors was retrieved experimentally by interacting with an actual HEOS system.
//!
//! Quick example:
//! ```
//! # use heos::ConnectError;
//! use heos::HeosConnection;
//! use heos::channel::Channel;
//! use heos::mock::{MockChannel, MockHeosSystem};
//! use parking_lot::Mutex;
//! use std::sync::Arc;
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), ConnectError> {
//! let system = Arc::new(Mutex::new(MockHeosSystem::default()));
//! let channel = MockChannel::new(system);
//! let heos = HeosConnection::from_channel(Channel::new(channel).await?).await?;
//! # Ok(())
//! # }
//! ```

use ahash::{HashMap, HashSet};
use async_trait::async_trait;
use educe::Educe;
use parking_lot::Mutex;
use serde_json::Value;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io::Result as IoResult;
use std::ops::{Index, RangeInclusive};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crate::channel::{ChannelBackend, ChannelState};
use crate::command::raw::RawCommand;
use crate::data::common::*;
use crate::data::group::*;
use crate::data::player::*;
use crate::data::quickselect::*;
use crate::data::response::*;
use crate::data::queue::*;
use crate::data::source::*;
use crate::state::group::GroupSnapshot;
use crate::state::player::{NowPlaying, NowPlayingProgress, PlayerSnapshot};

fn reflect_response_message(
    command: &RawCommand,
    extra: impl IntoIterator<Item=(String, String)>,
) -> String {
    let mut params = command.params().clone();
    for (k, v) in extra {
        params.insert(k, v);
    }
    params.to_string()
}

fn raw_message_response(
    command: &RawCommand,
    message: impl Into<String>,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(true),
            message: message.into(),
        },
        payload: None,
        options: None,
    }
}

fn success_response(
    command: &RawCommand,
    extra: impl IntoIterator<Item=(String, String)>,
    payload: Option<Value>,
    options: Option<Value>,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(true),
            message: reflect_response_message(command, extra),
        },
        payload,
        options,
    }
}

fn invalid_id_error(
    command: &RawCommand,
    id_type: impl Display,
    id_val: impl Display,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(false),
            message: format!("eid=2&text=Invalid {id_type} '{id_val}'"),
        },
        payload: None,
        options: None,
    }
}

fn missing_argument_error(
    command: &RawCommand,
    arg_name: impl Display,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(false),
            message: format!("eid=3&text=Missing '{arg_name}' argument"),
        },
        payload: None,
        options: None,
    }
}

fn invalid_argument_error(
    command: &RawCommand,
    arg_name: impl Display,
    error: impl Display,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(false),
            message: format!("eid=3&text=Invalid '{arg_name}' argument: {error}"),
        },
        payload: None,
        options: None,
    }
}

fn out_of_range_error(
    command: &RawCommand,
    idx: impl Debug,
    bounds: impl Debug,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(false),
            message: format!("eid=9&text=Index ({idx:?}) out of range: {bounds:?}"),
        },
        payload: None,
        options: None,
    }
}

fn internal_error(
    command: &RawCommand,
    error: impl Display,
) -> RawResponse {
    RawResponse {
        heos: RawResponseHeos {
            command: format!("{}/{}", command.group(), command.name()),
            result: Some(false),
            message: format!("eid=11&text=Internal error: {error}"),
        },
        payload: None,
        options: None,
    }
}

fn parse_command_argument<T>(
    command: &RawCommand,
    arg_name: impl AsRef<str>,
) -> Result<T, RawResponse>
where
    T: FromStr,
    T::Err: Display,
{
    let arg_name = arg_name.as_ref();
    match command.params().get(arg_name) {
        Some(arg_str) => T::from_str(arg_str.as_str())
            .map_err(|error| invalid_argument_error(command, arg_name, error)),
        None => Err(missing_argument_error(command, arg_name)),
    }
}

fn parse_command_argument_default<T>(
    command: &RawCommand,
    arg_name: impl AsRef<str>,
) -> Result<T, RawResponse>
where
    T: FromStr + Default,
    T::Err: Display,
{
    let arg_name = arg_name.as_ref();
    match command.params().get(arg_name) {
        Some(arg_str) => T::from_str(arg_str.as_str())
            .map_err(|error| invalid_argument_error(command, arg_name, error)),
        None => Ok(T::default()),
    }
}

fn parse_command_argument_option<T>(
    command: &RawCommand,
    arg_name: impl AsRef<str>,
) -> Result<Option<T>, RawResponse>
where
    T: FromStr,
    T::Err: Display,
{
    let arg_name = arg_name.as_ref();
    match command.params().get(arg_name) {
        Some(arg_str) => T::from_str(arg_str.as_str())
            .map(Some)
            .map_err(|error| invalid_argument_error(command, arg_name, error)),
        None => Ok(None),
    }
}

struct RangeWrapper(RangeInclusive<usize>);
impl FromStr for RangeWrapper {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (str_s, str_e) = s.split_once(',')
            .ok_or(format!("No ',' delimiter found in {s}"))?;
        let start = usize::from_str(str_s)
            .map_err(|err| err.to_string())?;
        let end = usize::from_str(str_e)
            .map_err(|err| err.to_string())?;
        Ok(Self(start..=end))
    }
}

struct VecWrapper<T>(Vec<T>);
impl<T> FromStr for VecWrapper<T>
where
    T: FromStr,
    T::Err: Display,
{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split(',')
            .map(|ss| T::from_str(ss).map_err(|err| format!("{err}")))
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}

/// Trait representing a mock data item.
///
/// This is used by e.g. [MockDataSet] to retrieve item IDs generically.
pub trait MockItem {
    type Id: Eq + Hash;

    /// The ID of this mock item.
    fn id(&self) -> Self::Id;
}

/// Mock around a [player](crate::data::player).
#[derive(Debug)]
pub struct MockPlayer {
    /// Player-specific data and information.
    pub snapshot: PlayerSnapshot,
    /// Quickselect slots for this player.
    pub quickselects: [QuickSelect; 6],
}

impl MockPlayer {
    /// Create a new mock player using [PlayerInfo].
    ///
    /// All other data in the mock player that is not contained in [PlayerInfo] is initialized to
    /// default values, and can be manipulated directly through fields afterward.
    pub fn new(info: PlayerInfo) -> Self {
        let snapshot = PlayerSnapshot {
            info,
            now_playing: NowPlaying {
                info: NowPlayingInfo::Song {
                    info: QueuedTrackInfo {
                        song: "".to_string(),
                        album: "".to_string(),
                        artist: "".to_string(),
                        image_url: None,
                        media_id: "".to_string(),
                        queue_id: QueueId::from(0),
                        album_id: None,
                    },
                    source_id: SourceId::HeosAuxInputs,
                },
                progress: NowPlayingProgress::new(Duration::from_secs(0)),
            },
            queue: vec![],
            play_state: PlayState::Stop,
            volume: Volume::try_from(100).unwrap(),
            mute: MuteState::Off,
            repeat: RepeatMode::Off,
            shuffle: ShuffleMode::Off,
        };
        let quickselects = [
            QuickSelect {
                id: QuickSelectId::try_from(1).unwrap(),
                name: "QuickSelect1".to_string(),
            },
            QuickSelect {
                id: QuickSelectId::try_from(2).unwrap(),
                name: "QuickSelect2".to_string(),
            },
            QuickSelect {
                id: QuickSelectId::try_from(3).unwrap(),
                name: "QuickSelect3".to_string(),
            },
            QuickSelect {
                id: QuickSelectId::try_from(4).unwrap(),
                name: "QuickSelect4".to_string(),
            },
            QuickSelect {
                id: QuickSelectId::try_from(5).unwrap(),
                name: "QuickSelect5".to_string(),
            },
            QuickSelect {
                id: QuickSelectId::try_from(6).unwrap(),
                name: "QuickSelect6".to_string(),
            },
        ];

        Self {
            snapshot,
            quickselects,
        }
    }

    // TODO: Do queue_ids need to be adjusted? Check to see what the actual HEOS
    //  system does with queue_ids
    fn adjust_queue_ids(&mut self) {
        match &mut self.snapshot.now_playing.info {
            NowPlayingInfo::Song { info, .. } | NowPlayingInfo::Station { info, .. } =>
                info.queue_id = QueueId::from(0)
        }
        for (idx, info) in &mut self.snapshot.queue.iter_mut().enumerate() {
            info.queue_id = QueueId::from(idx as u64);
        }
    }
}

impl MockItem for MockPlayer {
    type Id = PlayerId;

    #[inline]
    fn id(&self) -> Self::Id {
        self.snapshot.info.player_id
    }
}

/// Mock around a [group](crate::data::group).
#[derive(Debug)]
pub struct MockGroup {
    /// Group-specific data and information.
    pub snapshot: GroupSnapshot,
}

impl MockGroup {
    /// Create a new mock group using [GroupInfo].
    ///
    /// All other data in the mock group that is not contained in [GroupInfo] is initialized to
    /// default values, and can be manipulated directly through fields afterward.
    pub fn new(info: GroupInfo) -> Self {
        let leader_id = 'search: {
            for player in &info.players {
                if player.role == GroupRole::Leader {
                    break 'search player.player_id
                }
            }
            panic!("No leader found in {info:?}")
        };
        Self {
            snapshot: GroupSnapshot {
                info,
                leader_id,
                volume: Volume::try_from(100).unwrap(),
                mute: MuteState::Off,
            }
        }
    }
}

impl MockItem for MockGroup {
    type Id = GroupId;

    #[inline]
    fn id(&self) -> Self::Id {
        self.snapshot.info.group_id
    }
}

/// Mock around a [source](crate::data::source).
///
/// Sources are pre-populated for a [mock system](MockHeosSystem), and do not need to be manually
/// populated, aside from their [catalogues](MockSource::catalogue)
#[derive(Debug)]
pub struct MockSource {
    /// Source-specific data and information.
    pub info: SourceInfo,
    /// Any search criteria that are valid for this source.
    ///
    /// This is pre-populated for a [mock system](MockHeosSystem).
    pub search_criteria: Vec<SearchCriteria>,
    /// Catalogue of tracks/artists/albums/etc for this source.
    ///
    /// This is *NOT* pre-populated, and should be populated by your test if needed.
    pub catalogue: MockDataSet<MockTrack>,
}

impl MockSource {
    /// Create a new mock source using [SourceInfo].
    ///
    /// All other data in the mock source that is not contained in [SourceInfo] is initialized to
    /// default values, and can be manipulated directly through fields afterward.
    pub fn new(info: SourceInfo) -> Self {
        Self {
            info,
            search_criteria: vec![],
            catalogue: MockDataSet::default(),
        }
    }
}

impl MockItem for MockSource {
    type Id = SourceId;

    #[inline]
    fn id(&self) -> Self::Id {
        self.info.source_id
    }
}

/// Mock around a playlist of tracks.
#[derive(Debug)]
pub struct MockPlaylist {
    /// ID of this playlist.
    pub container_id: String,
    /// Name of this playlist
    pub name: String,
    /// Tracks in this playlist.
    pub tracks: Vec<QueuedTrackInfo>,
}

impl MockPlaylist {
    /// Create a new mock playlist with an empty set of tracks.
    pub fn new(container_id: String, name: String) -> Self {
        Self {
            container_id,
            name,
            tracks: vec![],
        }
    }
}

impl MockItem for MockPlaylist {
    type Id = String;

    #[inline]
    fn id(&self) -> Self::Id {
        self.container_id.clone()
    }
}

/// Mock around a track, for usage in source [catalogues](MockSource::catalogue).
#[derive(Debug, Clone)]
pub struct MockTrack {
    /// Track information.
    pub info: QueuedTrackInfo,
    /// ID of the source this track belongs to.
    pub source_id: SourceId,
    /// Duration of this track.
    pub duration: Duration,
}

impl MockTrack {
    /// Create a new mock track.
    pub fn new(info: QueuedTrackInfo, source_id: SourceId, duration: Duration) -> Self {
        Self {
            info,
            source_id,
            duration,
        }
    }
}

impl MockItem for MockTrack {
    type Id = String;

    #[inline]
    fn id(&self) -> Self::Id {
        self.info.media_id.clone()
    }
}

/// Set of mock data.
///
/// This functions like a hash set where the items are keyed by the ID specified by [MockItem]
/// instead of their own hash.
#[derive(Debug, Educe)]
#[educe(Default)]
pub struct MockDataSet<T: MockItem> {
    inner: HashMap<T::Id, T>,
}

impl<T: MockItem> MockDataSet<T> {
    #[inline]
    pub fn insert(&mut self, item: T) {
        self.inner.insert(item.id(), item);
    }

    #[inline]
    pub fn get(&self, id: &T::Id) -> Option<&T> {
        self.inner.get(id)
    }

    #[inline]
    pub fn get_mut(&mut self, id: &T::Id) -> Option<&mut T> {
        self.inner.get_mut(id)
    }

    #[inline]
    pub fn remove(&mut self, id: &T::Id) -> Option<T> {
        self.inner.remove(id)
    }

    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline]
    pub fn iter(&self) -> std::collections::hash_map::Values<'_, T::Id, T> {
        self.inner.values()
    }
}

impl<'a, T: MockItem> IntoIterator for &'a MockDataSet<T> {
    type Item = &'a T;
    type IntoIter = std::collections::hash_map::Values<'a, T::Id, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: MockItem> Index<&T::Id> for MockDataSet<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: &T::Id) -> &Self::Output {
        self.inner.index(index)
    }
}

fn get_default_sources() -> MockDataSet<MockSource> {
    let mut sources = MockDataSet::default();

    // All of these details were found experimentally by querying an actual HEOS system
    sources.insert(MockSource::new(SourceInfo {
        name: "Pandora".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/pandora.png").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::Pandora,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert({
        let mut rhapsody = MockSource::new(SourceInfo {
            name: "Rhapsody".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/rhapsody.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::Rhapsody,
            available: SourceAvailable::False,
            service_username: None,
        });
        rhapsody.search_criteria = vec![
            SearchCriteria {
                name: "Artist".to_string(),
                criteria: CriteriaId::Artist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Album".to_string(),
                criteria: CriteriaId::Album,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Track".to_string(),
                criteria: CriteriaId::Track,
                wildcard: false,
                playable: true,
                search_prefix: Some("SEARCHED_TRACKS-".to_string()),
            },
        ];
        rhapsody
    });
    sources.insert({
        let mut tune_in = MockSource::new(SourceInfo {
            name: "TuneIn".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/tunein.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::TuneIn,
            available: SourceAvailable::False,
            service_username: None,
        });
        tune_in.search_criteria = vec![
            SearchCriteria {
                name: "Station".to_string(),
                criteria: CriteriaId::Station,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
        ];
        tune_in
    });
    sources.insert(MockSource::new(SourceInfo {
        name: "Spotify".to_string(),
        image_url: Url::from_str("data:text/NoImageUrlHasBeenSet").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::Spotify,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert({
        let mut deezer = MockSource::new(SourceInfo {
            name: "Deezer".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/deezer.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::Deezer,
            available: SourceAvailable::False,
            service_username: None,
        });
        deezer.search_criteria = vec![
            SearchCriteria {
                name: "Artist".to_string(),
                criteria: CriteriaId::Artist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Album".to_string(),
                criteria: CriteriaId::Album,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Track".to_string(),
                criteria: CriteriaId::Track,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
        ];
        deezer
    });
    sources.insert(MockSource::new(SourceInfo {
        name: "Napster".to_string(),
        image_url: Url::from_str("data:text/NoImageUrlHasBeenSet").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::Napster,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert({
        let mut i_heart_radio = MockSource::new(SourceInfo {
            name: "iHeartRadio".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/iheartradio.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::IHeartRadio,
            available: SourceAvailable::False,
            service_username: None,
        });
        i_heart_radio.search_criteria = vec![
            SearchCriteria {
                name: "Artist".to_string(),
                criteria: CriteriaId::Artist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Shows".to_string(),
                criteria: CriteriaId::Shows,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Track".to_string(),
                criteria: CriteriaId::Track,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
        ];
        i_heart_radio
    });
    sources.insert(MockSource::new(SourceInfo {
        name: "Sirius XM".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/siriusxm.p
ng").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::SiriusXm,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert({
        let mut soundcloud = MockSource::new(SourceInfo {
            name: "Soundcloud".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/soundcloud.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::Soundcloud,
            available: SourceAvailable::False,
            service_username: None,
        });
        soundcloud.search_criteria = vec![
            SearchCriteria {
                name: "Accounts".to_string(),
                criteria: CriteriaId::Accounts,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Track".to_string(),
                criteria: CriteriaId::Track,
                wildcard: false,
                playable: true,
                search_prefix: Some("SEARCHED_TRACKS-".to_string()),
            },
        ];
        soundcloud
    });
    sources.insert({
        let mut tidal = MockSource::new(SourceInfo {
            name: "Tidal".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/tidal.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::Tidal,
            available: SourceAvailable::False,
            service_username: None,
        });
        tidal.search_criteria = vec![
            SearchCriteria {
                name: "Artist".to_string(),
                criteria: CriteriaId::Artist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Album".to_string(),
                criteria: CriteriaId::Album,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Track".to_string(),
                criteria: CriteriaId::Track,
                wildcard: false,
                playable: true,
                search_prefix: Some("SEARCHED_TRACKS-".to_string()),
            },
            SearchCriteria {
                name: "Playlist".to_string(),
                criteria: CriteriaId::Playlist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
        ];
        tidal
    });
    sources.insert(MockSource::new(SourceInfo {
        name: "Amazon Music".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/amazon
.png").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::AmazonMusic,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert(MockSource::new(SourceInfo {
        name: "Moodmix".to_string(),
        image_url: Url::from_str("data:text/NoImageUrlHasBeenSet").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::Moodmix,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert(MockSource::new(SourceInfo {
        name: "QQMusic".to_string(),
        image_url: Url::from_str("data:text/NoImageUrlHasBeenSet").unwrap(),
        source_type: SourceType::MusicService,
        source_id: SourceId::QQMusic,
        available: SourceAvailable::False,
        service_username: None,
    }));
    sources.insert({
        let mut qobuz = MockSource::new(SourceInfo {
            name: "Qobuz".to_string(),
            image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/qobuz.png").unwrap(),
            source_type: SourceType::MusicService,
            source_id: SourceId::Qobuz,
            available: SourceAvailable::False,
            service_username: None,
        });
        qobuz.search_criteria = vec![
            SearchCriteria {
                name: "Artist".to_string(),
                criteria: CriteriaId::Artist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Album".to_string(),
                criteria: CriteriaId::Album,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
            SearchCriteria {
                name: "Track".to_string(),
                criteria: CriteriaId::Track,
                wildcard: false,
                playable: true,
                search_prefix: Some("SEARCHED_TRACKS-".to_string()),
            },
            SearchCriteria {
                name: "Playlist".to_string(),
                criteria: CriteriaId::Playlist,
                wildcard: false,
                playable: false,
                search_prefix: None,
            },
        ];
        qobuz
    });
    sources.insert(MockSource::new(SourceInfo {
        name: "Local Music".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/musicsource_logo_servers.png").unwrap(),
        source_type: SourceType::HeosServer,
        source_id: SourceId::LocalUsbOrDlna,
        available: SourceAvailable::True,
        service_username: None,
    }));
    sources.insert(MockSource::new(SourceInfo {
        name: "HEOS Playlists".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/musicsource_logo_playlists.png").unwrap(),
        source_type: SourceType::HeosService,
        source_id: SourceId::HeosPlaylists,
        available: SourceAvailable::True,
        service_username: None,
    }));
    sources.insert(MockSource::new(SourceInfo {
        name: "HEOS History".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/musicsource_logo_history.png").unwrap(),
        source_type: SourceType::HeosService,
        source_id: SourceId::HeosHistory,
        available: SourceAvailable::True,
        service_username: None,
    }));
    sources.insert(MockSource::new(SourceInfo {
        name: "HEOS AUX Inputs".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/musicsource_logo_aux.png").unwrap(),
        source_type: SourceType::HeosService,
        source_id: SourceId::HeosAuxInputs,
        available: SourceAvailable::True,
        service_username: None,
    }));
    sources.insert(MockSource::new(SourceInfo {
        name: "HEOS Favorites".to_string(),
        image_url: Url::from_str("https://production.ws.skyegloup.com:443/media/images/service/logos/musicsource_logo_favorites.png").unwrap(),
        source_type: SourceType::HeosService,
        source_id: SourceId::HeosFavorites,
        available: SourceAvailable::True,
        service_username: None,
    }));

    sources
}

/// The mock HEOS system.
///
/// A new system can be created via `MockHeosSystem::default()`, and can be populated with
/// test-relevant data afterward.
#[derive(Debug, Educe)]
#[educe(Default)]
pub struct MockHeosSystem {
    /// Signed-in username of this system.
    ///
    /// Defaults to `None`.
    pub username: Option<String>,
    /// Set of players for this system.
    ///
    /// Defaults to empty.
    pub players: MockDataSet<MockPlayer>,
    /// Set of groups for this system.
    ///
    /// Defaults to empty.
    pub groups: MockDataSet<MockGroup>,
    /// Set of sources for this system.
    ///
    /// Pre-populates with a known set of valid HEOS sources based on experimentation and the CLI
    /// spec. Does not populate source [catalogues](MockSource::catalogue) however; those will need
    /// to be populated by the test.
    #[educe(Default(expression = get_default_sources()))]
    pub sources: MockDataSet<MockSource>,
    /// Set of playlists for this system.
    ///
    /// Defaults to empty.
    // TODO: Move playlists to "Heos Playlists" mock source?
    pub playlists: MockDataSet<MockPlaylist>,
    next_playlist_num: u64,
}

impl MockHeosSystem {
    fn player_for_command(&mut self, command: &RawCommand) -> Result<&mut MockPlayer, RawResponse> {
        let player_id = parse_command_argument(command, "pid")?;
        match self.players.get_mut(&player_id) {
            Some(player) => Ok(player),
            None => Err(invalid_id_error(command, "pid", player_id)),
        }
    }

    fn group_for_command(&mut self, command: &RawCommand) -> Result<&mut MockGroup, RawResponse> {
        let group_id = parse_command_argument(command, "gid")?;
        match self.groups.get_mut(&group_id) {
            Some(group) => Ok(group),
            None => Err(invalid_id_error(command, "gid", group_id)),
        }
    }

    fn source_for_command(&mut self, command: &RawCommand) -> Result<&mut MockSource, RawResponse> {
        let source_id = parse_command_argument(command, "sid")?;
        match self.sources.get_mut(&source_id) {
            Some(source) => Ok(source),
            None => Err(invalid_id_error(command, "sid", source_id)),
        }
    }
}

/// Channel implementation for a mock system.
///
/// This is what allows the mock system to be used with [connections](crate::HeosConnection).
#[derive(Debug)]
pub struct MockChannel {
    state: Option<Arc<Mutex<ChannelState>>>,
    system: Arc<Mutex<MockHeosSystem>>,
}

impl MockChannel {
    /// Create a new channel.
    pub fn new(system: Arc<Mutex<MockHeosSystem>>) -> Self {
        Self {
            state: None,
            system,
        }
    }

    fn response_for_command(&mut self, command: RawCommand) -> Result<RawResponse, RawResponse> {
        println!("{command}");
        let group = command.group();
        let name = command.name();

        let mut system = self.system.lock();

        let response = match (group, name) {
            ("system", "register_for_change_events") => {
                match command.params().get("enable") {
                    Some(val) => match val.as_str() {
                        "on" | "off" => success_response(&command, [], None, None),
                        _ =>  invalid_argument_error(
                            &command,
                            "enable",
                            format!("{val} is not one of 'on'|'off'"),
                        ),
                    },
                    None => missing_argument_error(&command, "enable"),
                }
            },
            ("system", "check_account") => {
                match &system.username {
                    Some(username) =>
                        raw_message_response(&command, format!("signed_in&un={username}")),
                    None => raw_message_response(&command, "signed_out"),
                }
            },
            ("system", "sign_in") => {
                let username = parse_command_argument::<String>(&command, "username")?;
                let _ = parse_command_argument::<String>(&command, "password")?;
                system.username = Some(username.clone());
                raw_message_response(&command, format!("signed_in&un={username}"))
            },
            ("system", "sign_out") => {
                system.username = None;
                raw_message_response(&command, "signed_out")
            },
            ("system", "heart_beat") |
            ("system", "reboot") => success_response(&command, [], None, None),
            ("player", "get_players") => {
                let players = system.players.iter()
                    .map(|player| player.snapshot.info.clone())
                    .collect::<Vec<_>>();
                let payload = serde_json::to_value(players)
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("player", "get_player_info") => {
                let player = system.player_for_command(&command)?;
                let payload = serde_json::to_value(player.snapshot.info.clone())
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("player", "get_play_state") => {
                let player = system.player_for_command(&command)?;
                success_response(&command, [
                    ("state".to_string(), player.snapshot.play_state.to_string()),
                ], None, None)
            },
            ("player", "set_play_state") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.play_state = parse_command_argument::<PlayState>(&command, "state")?;
                success_response(&command, [], None, None)
            },
            ("player", "get_now_playing_media") => {
                let player = system.player_for_command(&command)?;
                let payload = serde_json::to_value(player.snapshot.now_playing.info.clone())
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("player", "get_volume") => {
                let player = system.player_for_command(&command)?;
                success_response(&command, [
                    ("level".to_string(), player.snapshot.volume.to_string()),
                ], None, None)
            },
            ("player", "set_volume") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.volume = parse_command_argument::<Volume>(&command, "level")?;
                success_response(&command, [], None, None)
            },
            ("player", "volume_up") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.volume.saturating_add(parse_command_argument_default::<VolumeStep>(&command, "step")?);
                success_response(&command, [], None, None)
            },
            ("player", "volume_down") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.volume.saturating_sub(parse_command_argument_default::<VolumeStep>(&command, "step")?);
                success_response(&command, [], None, None)
            },
            ("player", "get_mute") => {
                let player = system.player_for_command(&command)?;
                success_response(&command, [
                    ("state".to_string(), player.snapshot.mute.to_string()),
                ], None, None)
            },
            ("player", "set_mute") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.mute = parse_command_argument::<MuteState>(&command, "state")?;
                success_response(&command, [], None, None)
            },
            ("player", "toggle_mute") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.mute = match player.snapshot.mute {
                    MuteState::Off => MuteState::On,
                    MuteState::On => MuteState::Off,
                };
                success_response(&command, [], None, None)
            },
            ("player", "get_play_mode") => {
                let player = system.player_for_command(&command)?;
                success_response(&command, [
                    ("repeat".to_string(), player.snapshot.repeat.to_string()),
                    ("shuffle".to_string(), player.snapshot.shuffle.to_string()),
                ], None, None)
            },
            ("player", "set_play_mode") => {
                let player = system.player_for_command(&command)?;
                if let Some(repeat) = parse_command_argument_option::<RepeatMode>(&command, "repeat")? {
                    player.snapshot.repeat = repeat;
                }
                if let Some(shuffle) = parse_command_argument_option::<ShuffleMode>(&command, "shuffle")? {
                    player.snapshot.shuffle = shuffle;
                }
                success_response(&command, [], None, None)
            },
            ("player", "get_queue") => {
                let player = system.player_for_command(&command)?;
                let queue = match parse_command_argument_option::<RangeWrapper>(&command, "range")? {
                    Some(range) => {
                        player.snapshot.queue.get(range.0.clone())
                            .ok_or_else(|| out_of_range_error(&command, range.0, 0..player.snapshot.queue.len()))?
                            .into_iter()
                            .map(Clone::clone)
                            .collect::<Vec<_>>()
                    },
                    None => player.snapshot.queue.clone(),
                };
                let payload = serde_json::to_value(queue)
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("player", "play_queue") => {
                let player = system.player_for_command(&command)?;
                let queue_id = parse_command_argument::<QueueId>(&command, "qid")?;
                if (*queue_id as usize) < player.snapshot.queue.len() {
                    player.snapshot.queue = player.snapshot.queue.split_off(*queue_id as usize);
                    player.adjust_queue_ids();
                    success_response(&command, [], None, None)
                } else {
                    out_of_range_error(&command, queue_id, 0..player.snapshot.queue.len())
                }
            },
            ("player", "remove_from_queue") => {
                let player = system.player_for_command(&command)?;
                let mut queue_ids = parse_command_argument::<VecWrapper<QueueId>>(&command, "qid")?.0;
                queue_ids.sort_by_cached_key(|id| **id);
                for qid in queue_ids.into_iter().rev() {
                    if (*qid as usize) < player.snapshot.queue.len() {
                        player.snapshot.queue.remove(*qid as usize);
                        player.adjust_queue_ids();
                    } else {
                        return Err(out_of_range_error(&command, qid, 0..player.snapshot.queue.len()))
                    }
                }
                success_response(&command, [], None, None)
            },
            ("player", "save_queue") => {
                let player = system.player_for_command(&command)?;
                let tracks = player.snapshot.queue.clone();
                let playlist_id = format!("playlist-{}", system.next_playlist_num);
                let playlist_name = parse_command_argument::<String>(&command, "name")?;
                system.next_playlist_num += 1;
                let mut playlist = MockPlaylist::new(playlist_id, playlist_name);
                playlist.tracks = tracks;
                system.playlists.insert(playlist);
                success_response(&command, [], None, None)
            },
            ("player", "clear_queue") => {
                let player = system.player_for_command(&command)?;
                player.snapshot.queue.clear();
                success_response(&command, [], None, None)
            },
            ("player", "move_queue_item") => {
                let player = system.player_for_command(&command)?;
                let src_queue_ids = parse_command_argument::<VecWrapper<QueueId>>(&command, "sqid")?.0
                    .into_iter()
                    .collect::<HashSet<QueueId>>();
                let dst_queue_id = parse_command_argument::<QueueId>(&command, "dqid")?;

                let mut old_queue = Vec::with_capacity(player.snapshot.queue.len());
                std::mem::swap(&mut old_queue, &mut player.snapshot.queue);

                let mut pre = Vec::with_capacity(*dst_queue_id as usize);
                let mut mid = Vec::with_capacity(src_queue_ids.len());
                let mut post = Vec::with_capacity(old_queue.len() - *dst_queue_id as usize);
                let mut past_mid = false;
                for item in old_queue {
                    if item.queue_id == dst_queue_id {
                        past_mid = true;
                    }

                    if src_queue_ids.contains(&item.queue_id) {
                        mid.push(item);
                    } else if past_mid {
                        post.push(item);
                    } else {
                        pre.push(item);
                    }
                }
                player.snapshot.queue.extend(pre);
                player.snapshot.queue.extend(mid);
                player.snapshot.queue.extend(post);
                player.adjust_queue_ids();

                success_response(&command, [], None, None)
            },
            ("player", "play_next") => {
                let player = system.player_for_command(&command)?;

                let song = player.snapshot.queue.remove(0);
                match &mut player.snapshot.now_playing.info {
                    NowPlayingInfo::Song { info, .. } => *info = song,
                    NowPlayingInfo::Station { info, .. } => *info = song,
                };
                player.adjust_queue_ids();

                success_response(&command, [], None, None)
            },
            ("player", "play_previous") => {
                let _ = system.player_for_command(&command)?;
                // TODO: keep track of history
                success_response(&command, [], None, None)
            },
            ("player", "set_quickselect") => {
                let _ = system.player_for_command(&command)?;
                let _ = parse_command_argument::<QuickSelectId>(&command, "id")?;
                // quickselects use HEOS information not available in the mock
                success_response(&command, [], None, None)
            },
            ("player", "play_quickselect") => {
                let _ = system.player_for_command(&command)?;
                let _ = parse_command_argument::<QuickSelectId>(&command, "id")?;
                // quickselects use HEOS information not available in the mock
                success_response(&command, [], None, None)
            },
            ("player", "get_quickselects") => {
                let player = system.player_for_command(&command)?;
                let id = parse_command_argument_option::<QuickSelectId>(&command, "id")?;

                let quickselects = match id {
                    Some(id) => vec![player.quickselects[*id as usize].clone()],
                    None => player.quickselects.iter().cloned().collect::<Vec<_>>(),
                };
                let payload = serde_json::to_value(quickselects)
                    .map_err(|error| internal_error(&command, error))?;

                success_response(&command, [], Some(payload), None)
            },
            ("player", "check_update") => {
                let _ = system.player_for_command(&command)?;
                let payload = serde_json::to_value(UpdatePayload {
                    update: UpdateAvailable::None,
                }).map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("group", "get_groups") => {
                let groups = system.groups.iter()
                    .map(|group| group.snapshot.info.clone())
                    .collect::<Vec<_>>();
                let payload = serde_json::to_value(groups)
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("group", "get_group_info") => {
                let group = system.group_for_command(&command)?;
                let payload = serde_json::to_value(group.snapshot.info.clone())
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("group", "set_group") => {
                let player_ids = parse_command_argument::<VecWrapper<PlayerId>>(&command, "pid")?.0;
                if player_ids.is_empty() {
                    invalid_id_error(&command, "pid", "")
                } else {
                    let leader_id = player_ids[0];
                    let mut group_id = None;
                    for group in &system.groups {
                        if group.snapshot.leader_id == leader_id {
                            group_id = Some(group.snapshot.info.group_id);
                        }
                    }

                    if player_ids.len() == 1 {
                        match group_id {
                            Some(group_id) => {
                                system.groups.remove(&group_id);
                                success_response(&command, [], None, None)
                            },
                            None => invalid_id_error(&command, "pid", leader_id),
                        }
                    } else {
                        let player_ids = player_ids[1..].iter()
                            .cloned()
                            .collect::<HashSet<_>>();

                        let leader = {
                            let leader = system.players.get(&leader_id)
                                .ok_or(invalid_id_error(&command, "pid", leader_id))?;
                            GroupPlayer {
                                name: leader.snapshot.info.name.clone(),
                                player_id: leader_id,
                                role: GroupRole::Leader,
                            }
                        };
                        let players = std::iter::once(Ok(leader))
                            .chain(player_ids.into_iter()
                                .map(|player_id| {
                                    let player = system.players.get(&player_id)
                                        .ok_or(invalid_id_error(&command, "pid", player_id))?;
                                    Ok(GroupPlayer {
                                        name: player.snapshot.info.name.clone(),
                                        player_id,
                                        role: GroupRole::Member,
                                    })
                                })
                            )
                            .collect::<Result<Vec<_>, _>>()?;

                        let name = players.iter()
                            .map(|player| player.name.as_str())
                            .collect::<Vec<_>>()
                            .join("+");

                        let group_id = match group_id {
                            Some(group_id) => {
                                let group = system.groups.get_mut(&group_id).unwrap();
                                group.snapshot.info.name = name.clone();
                                group.snapshot.info.players = players;
                                group_id
                            },
                            None => {
                                // Just use the leader's ID, as the leader is the anchor of the group
                                let group_id = GroupId::from(leader_id.0);
                                let group = MockGroup::new(GroupInfo {
                                    name: name.clone(),
                                    group_id,
                                    players,
                                });
                                system.groups.insert(group);
                                group_id
                            },
                        };

                        success_response(&command, [
                            ("gid".to_string(), group_id.to_string()),
                            ("name".to_string(), name),
                        ], None, None)
                    }
                }
            },
            ("group", "get_volume") => {
                let group = system.group_for_command(&command)?;
                success_response(&command, [
                    ("level".to_string(), group.snapshot.volume.to_string()),
                ], None, None)
            },
            ("group", "set_volume") => {
                let group = system.group_for_command(&command)?;
                group.snapshot.volume = parse_command_argument::<Volume>(&command, "level")?;
                success_response(&command, [], None, None)
            },
            ("group", "volume_up") => {
                let group = system.group_for_command(&command)?;
                group.snapshot.volume.saturating_add(parse_command_argument_default::<VolumeStep>(&command, "step")?);
                success_response(&command, [], None, None)
            },
            ("group", "volume_down") => {
                let group = system.group_for_command(&command)?;
                group.snapshot.volume.saturating_sub(parse_command_argument_default::<VolumeStep>(&command, "step")?);
                success_response(&command, [], None, None)
            },
            ("group", "get_mute") => {
                let group = system.group_for_command(&command)?;
                success_response(&command, [
                    ("state".to_string(), group.snapshot.mute.to_string()),
                ], None, None)
            },
            ("group", "set_mute") => {
                let group = system.group_for_command(&command)?;
                group.snapshot.mute = parse_command_argument::<MuteState>(&command, "state")?;
                success_response(&command, [], None, None)
            },
            ("group", "toggle_mute") => {
                let group = system.group_for_command(&command)?;
                group.snapshot.mute = match group.snapshot.mute {
                    MuteState::Off => MuteState::On,
                    MuteState::On => MuteState::Off,
                };
                success_response(&command, [], None, None)
            },
            ("browse", "get_music_sources") => {
                let sources = system.sources.iter()
                    .map(|source| source.info.clone())
                    .collect::<Vec<_>>();
                let payload = serde_json::to_value(sources)
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("browse", "get_source_info") => {
                let source = system.source_for_command(&command)?;
                let payload = serde_json::to_value(source.info.clone())
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("browse", "get_search_criteria") => {
                let source = system.source_for_command(&command)?;
                let payload = serde_json::to_value(source.search_criteria.clone())
                    .map_err(|error| internal_error(&command, error))?;
                success_response(&command, [], Some(payload), None)
            },
            ("browse", "add_to_queue") => {
                let source = system.source_for_command(&command)?;
                let media_id = parse_command_argument::<String>(&command, "mid")?;
                let _container_id = parse_command_argument_option::<String>(&command, "cid")?;
                let track = match source.catalogue.get(&media_id) {
                    Some(track) => track.clone(),
                    None => return Err(invalid_id_error(&command, "mid", media_id)),
                };

                let player = system.player_for_command(&command)?;
                let add_to_queue_type = parse_command_argument::<AddToQueueType>(&command, "aid")?;
                match add_to_queue_type {
                    AddToQueueType::PlayNow => {
                        player.snapshot.now_playing = NowPlaying {
                            info: NowPlayingInfo::Song {
                                info: track.info.clone(),
                                source_id: track.source_id,
                            },
                            progress: NowPlayingProgress::new(track.duration),
                        };
                    },
                    AddToQueueType::PlayNext => {
                        player.snapshot.queue.insert(0, track.info.clone());
                    },
                    AddToQueueType::AddToEnd => {
                        player.snapshot.queue.push(track.info.clone());
                    },
                    AddToQueueType::ReplaceAndPlay => {
                        player.snapshot.now_playing = NowPlaying {
                            info: NowPlayingInfo::Song {
                                info: track.info.clone(),
                                source_id: track.source_id,
                            },
                            progress: NowPlayingProgress::new(track.duration),
                        };
                        player.snapshot.queue.clear();
                    },
                }
                player.adjust_queue_ids();

                success_response(&command, [], None, None)
            },
            ("browse", "rename_playlist") => {
                let source_id = parse_command_argument::<SourceId>(&command, "sid")?;
                if source_id != SourceId::HeosPlaylists {
                    return Err(invalid_argument_error(
                        &command,
                        "sid",
                        "Only HeosPlaylists (1025) is currently a valid ID",
                    ))
                }
                let container_id = parse_command_argument::<String>(&command, "cid")?;
                let playlist = match system.playlists.get_mut(&container_id) {
                    Some(playlist) => playlist,
                    None => return Err(invalid_id_error(&command, "cid", container_id)),
                };
                playlist.name = parse_command_argument::<String>(&command, "name")?;
                success_response(&command, [], None, None)
            },
            ("browse", "delete_playlist") => {
                let source_id = parse_command_argument::<SourceId>(&command, "sid")?;
                if source_id != SourceId::HeosPlaylists {
                    return Err(invalid_argument_error(
                        &command,
                        "sid",
                        "Only HeosPlaylists (1025) is currently a valid ID",
                    ))
                }
                let container_id = parse_command_argument::<String>(&command, "cid")?;
                if system.playlists.remove(&container_id).is_none() {
                    return Err(invalid_id_error(&command, "cid", container_id))
                }
                success_response(&command, [], None, None)
            },
            (_, _) => RawResponse {
                heos: RawResponseHeos {
                    command: format!("{group}/{name}"),
                    result: Some(false),
                    message: format!("eid=1&text=Unrecognized command {group}/{name}"),
                },
                payload: None,
                options: None,
            },
        };

        Ok(response)
    }
}

#[async_trait]
impl ChannelBackend for MockChannel {
    async fn init(&mut self, state: Arc<Mutex<ChannelState>>) -> IoResult<()> {
        self.state = Some(state);
        Ok(())
    }

    async fn send(&mut self, command: RawCommand) -> IoResult<()> {
        let response = match self.response_for_command(command) {
            Ok(response) | Err(response) => response,
        };

        let state = match &self.state {
            Some(state) => state,
            None => return Ok(()),
        };

        state.lock().handle_response(response);

        Ok(())
    }
}