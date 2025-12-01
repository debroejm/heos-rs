use parking_lot::Mutex;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::data::group::*;
use crate::data::player::*;
use crate::data::queue::*;
use crate::data::source::*;
use crate::mock::{MockChannel, MockGroup, MockHeosSystem, MockPlayer, MockPlaylist, MockTrack};
use crate::state::player::{NowPlaying, NowPlayingProgress};

fn create_doctest_channel() -> MockChannel {
    let mut system = MockHeosSystem::default();

    system.players.insert(MockPlayer::new(PlayerInfo {
        name: "Player1".to_string(),
        player_id: PlayerId::from(1),
        group_id: Some(GroupId::from(1)),
        model: "BasicPlayer".to_string(),
        version: "1".to_string(),
        ip: Ipv4Addr::new(192, 168, 0, 1).into(),
        network: NetworkType::Wired,
        line_out: LineOutLevelType::None,
        line_out_control: None,
        serial: None,
    }));
    system.players.insert(MockPlayer::new(PlayerInfo {
        name: "Player2".to_string(),
        player_id: PlayerId::from(2),
        group_id: Some(GroupId::from(1)),
        model: "BasicPlayer".to_string(),
        version: "1".to_string(),
        ip: Ipv4Addr::new(192, 168, 0, 2).into(),
        network: NetworkType::WiFi,
        line_out: LineOutLevelType::Fixed,
        line_out_control: Some(LineOutLevelControlType::Network),
        serial: None,
    }));
    system.players.insert(MockPlayer::new(PlayerInfo {
        name: "Player3".to_string(),
        player_id: PlayerId::from(3),
        group_id: Some(GroupId::from(1)),
        model: "BasicPlayer".to_string(),
        version: "1".to_string(),
        ip: Ipv4Addr::new(192, 168, 0, 3).into(),
        network: NetworkType::Unknown,
        line_out: LineOutLevelType::Variable,
        line_out_control: None,
        serial: None,
    }));

    system.groups.insert(MockGroup::new(GroupInfo {
        name: "Group1".to_string(),
        group_id: GroupId::from(1),
        players: vec![
            GroupPlayer {
                name: "Player1".to_string(),
                player_id: PlayerId::from(1),
                role: GroupRole::Leader,
            },
            GroupPlayer {
                name: "Player2".to_string(),
                player_id: PlayerId::from(2),
                role: GroupRole::Member,
            },
            GroupPlayer {
                name: "Player3".to_string(),
                player_id: PlayerId::from(3),
                role: GroupRole::Member,
            },
        ]
    }));

    let mut player = MockPlayer::new(PlayerInfo {
        name: "Player42".to_string(),
        player_id: PlayerId::from(42),
        group_id: None,
        model: "AQuestion".to_string(),
        version: "Everything".to_string(),
        ip: Ipv4Addr::new(192, 168, 0, 42).into(),
        network: NetworkType::Wired,
        line_out: LineOutLevelType::Variable,
        line_out_control: None,
        serial: None,
    });
    player.snapshot.now_playing = NowPlaying {
        info: NowPlayingInfo::Song {
            info: QueuedTrackInfo {
                song: "TheCurrentSong".to_string(),
                album: "TheCurrentAlbum".to_string(),
                artist: "TheCurrentArtist".to_string(),
                image_url: None,
                media_id: "current-media-id".to_string(),
                queue_id: QueueId::from(0),
                album_id: Some("current-album-id".to_string()),
            },
            source_id: SourceId::from(1),
        },
        progress: NowPlayingProgress::new(Duration::from_secs(9001)),
    };
    player.snapshot.queue = vec![
        QueuedTrackInfo {
            song: "Song1".to_string(),
            album: "QueuedSongs".to_string(),
            artist: "QueuedArtist".to_string(),
            image_url: None,
            media_id: "queued-song-1".to_string(),
            queue_id: QueueId::from(1),
            album_id: Some("queued-album-id".to_string()),
        },
        QueuedTrackInfo {
            song: "Song2".to_string(),
            album: "QueuedSongs".to_string(),
            artist: "QueuedArtist".to_string(),
            image_url: None,
            media_id: "queued-song-2".to_string(),
            queue_id: QueueId::from(2),
            album_id: Some("queued-album-id".to_string()),
        },
        QueuedTrackInfo {
            song: "Song3".to_string(),
            album: "QueuedSongs".to_string(),
            artist: "QueuedArtist".to_string(),
            image_url: None,
            media_id: "queued-song-3".to_string(),
            queue_id: QueueId::from(3),
            album_id: Some("queued-album-id".to_string()),
        },
        QueuedTrackInfo {
            song: "Song4".to_string(),
            album: "QueuedSongs".to_string(),
            artist: "QueuedArtist".to_string(),
            image_url: None,
            media_id: "queued-song-4".to_string(),
            queue_id: QueueId::from(4),
            album_id: Some("queued-album-id".to_string()),
        },
    ];
    system.players.insert(player);
    system.players.insert(MockPlayer::new(PlayerInfo {
        name: "Player43".to_string(),
        player_id: PlayerId::from(43),
        group_id: None,
        model: "AQuestion".to_string(),
        version: "Everything".to_string(),
        ip: Ipv4Addr::new(192, 168, 0, 43).into(),
        network: NetworkType::WiFi,
        line_out: LineOutLevelType::None,
        line_out_control: None,
        serial: None,
    }));

    let mut playlist = MockPlaylist::new("playlist-id".to_string(), "My Playlist".to_string());
    playlist.tracks.push(QueuedTrackInfo {
        song: "PlaylistSong1".to_string(),
        album: "PlaylistSongs".to_string(),
        artist: "PlaylistArtist".to_string(),
        image_url: None,
        media_id: "playlist-song-1".to_string(),
        // TODO: Should this have a queue_id?
        queue_id: QueueId::from(0),
        album_id: Some("playlist-album-id".to_string()),
    });
    system.playlists.insert(playlist);

    let local_usb = system.sources.get_mut(&SourceId::LocalUsbOrDlna).unwrap();
    local_usb.catalogue.insert(MockTrack::new(
        QueuedTrackInfo {
            song: "NewSong".to_string(),
            album: "".to_string(),
            artist: "NewArtist".to_string(),
            image_url: None,
            media_id: "new-song-id".to_string(),
            queue_id: QueueId::from(0),
            album_id: None,
        },
        SourceId::LocalUsbOrDlna,
        Duration::from_secs(123),
    ));

    let system = Arc::new(Mutex::new(system));
    MockChannel::new(system)
}

static DOCTEST_FLAG: AtomicBool = AtomicBool::new(false);

#[doc(hidden)]
pub fn install_doctest_handler() {
    DOCTEST_FLAG.store(true, Ordering::Relaxed);
}

pub(crate) fn try_doctest_channel() -> Option<MockChannel> {
    if DOCTEST_FLAG.load(Ordering::Relaxed) {
        Some(create_doctest_channel())
    } else {
        None
    }
}