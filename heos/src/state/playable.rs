use ahash::HashSet;
use educe::Educe;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use url::Url;

use crate::command::CommandError;
use crate::data::common::*;
use crate::data::group::*;
use crate::data::player::*;
use crate::data::song::SongInfo;
use crate::data::source::SourceId;
use crate::state::group::{Group, GroupsIter};
use crate::state::player::{NowPlaying, Player, PlayersIter, Queue};
use crate::state::State;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayableId {
    Player(PlayerId),
    Group(GroupId),
}

// Custom Debug impl so that pretty printing doesn't add unnecessary line breaks
impl Debug for PlayableId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player(id) => write!(f, "PlayableId({id:?})"),
            Self::Group(id) => write!(f, "PlayableId({id:?})"),
        }
    }
}

impl Display for PlayableId {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player(id) => Display::fmt(id, f),
            Self::Group(id) => Display::fmt(id, f),
        }
    }
}

impl From<PlayerId> for PlayableId {
    #[inline]
    fn from(id: PlayerId) -> Self {
        Self::Player(id)
    }
}

impl From<GroupId> for PlayableId {
    #[inline]
    fn from(id: GroupId) -> Self {
        Self::Group(id)
    }
}

#[derive(Debug, Clone)]
pub enum PlayableInfo {
    Player(PlayerInfo),
    Group(GroupInfo),
}

#[derive(Debug, Clone)]
pub struct PlayableSnapshot {
    pub id: PlayableId,
    pub info: PlayableInfo,
    pub now_playing: NowPlaying,
    pub queue: Vec<SongInfo>,
    pub play_state: PlayState,
    pub volume: Volume,
    pub mute: MuteState,
    pub repeat: RepeatMode,
    pub shuffle: ShuffleMode,
}

#[derive(Educe)]
#[educe(Deref)]
pub struct PlayableGroup<'a> {
    #[educe(Deref)]
    group: Group<'a>,
    leader: Player<'a>,
}

pub enum Playable<'a> {
    Player(Player<'a>),
    Group(PlayableGroup<'a>),
}

impl<'a> From<Player<'a>> for Playable<'a> {
    #[inline]
    fn from(player: Player<'a>) -> Self {
        Self::Player(player)
    }
}

macro_rules! delegate_player {
    ($fn_name:ident, $ret_type:ty) => {
        pub async fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name().await,
                Self::Group(group) => group.leader.$fn_name().await,
            }
        }
    };
    ($fn_name:ident, $($param:ident: $param_t:ty),*) => {
        pub async fn $fn_name(&self, $($param: $param_t),*) -> Result<(), CommandError> {
            match self {
                Self::Player(player) => player.$fn_name($($param),*).await,
                Self::Group(group) => group.leader.$fn_name($($param),*).await,
            }
        }
    };
}

macro_rules! delegate_both {
    ($fn_name:ident, $ret_type:ty) => {
        pub async fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name().await,
                Self::Group(group) => group.group.$fn_name().await,
            }
        }
    };
    ($fn_name:ident, $($param:ident: $param_t:ty),*) => {
        pub async fn $fn_name(&self, $($param: $param_t),*) -> Result<(), CommandError> {
            match self {
                Self::Player(player) => player.$fn_name($($param),*).await,
                Self::Group(group) => group.group.$fn_name($($param),*).await,
            }
        }
    };
}

impl<'a> Playable<'a> {
    #[inline]
    pub fn from_group(group: Group<'a>, leader: Player<'a>) -> Self {
        Self::Group(PlayableGroup {
            group,
            leader,
        })
    }

    pub fn id(&self) -> PlayableId {
        match self {
            Self::Player(player) => player.info().player_id.into(),
            Self::Group(group) => group.info().group_id.into(),
        }
    }

    pub fn info(&self) -> PlayableInfo {
        match self {
            Self::Player(player) => PlayableInfo::Player(player.info().clone()),
            Self::Group(group) => PlayableInfo::Group(group.info().clone()),
        }
    }

    delegate_player!(now_playing, NowPlaying);

    pub fn queue(&self) -> Queue<'_> {
        match self {
            Self::Player(player) => player.queue(),
            Self::Group(group) => group.leader.queue(),
        }
    }

    delegate_player!(play_state, PlayState);
    delegate_player!(set_play_state, state: PlayState);

    delegate_both!(volume, Volume);
    delegate_both!(set_volume, level: Volume);
    delegate_both!(volume_up, step: Option<VolumeStep>);
    delegate_both!(volume_down, step: Option<VolumeStep>);

    delegate_both!(mute, MuteState);
    delegate_both!(set_mute, state: MuteState);
    delegate_both!(toggle_mute, );

    delegate_player!(repeat, RepeatMode);
    delegate_player!(shuffle, ShuffleMode);
    delegate_player!(set_play_mode, repeat: Option<RepeatMode>, shuffle: Option<ShuffleMode>);

    delegate_player!(play_next, );
    delegate_player!(play_previous, );
    delegate_player!(
        play_station,
        source_id: SourceId,
        container_id: Option<String>,
        media_id: impl Into<String>,
        name: impl Into<String>
    );
    delegate_player!(play_preset, preset: usize);
    delegate_player!(play_input_source, src_player_id: Option<PlayerId>, input: impl Into<String>);
    delegate_player!(play_url, url: impl Into<Url>);

    delegate_player!(
        add_to_queue,
        source_id: SourceId,
        container_id: Option<String>,
        media_id: impl Into<String>,
        add_to_queue_type: AddToQueueType
    );

    pub async fn snapshot(&self) -> PlayableSnapshot {
        PlayableSnapshot {
            id: self.id(),
            info: self.info(),
            now_playing: self.now_playing().await,
            queue: self.queue().data().await.clone(),
            play_state: self.play_state().await,
            volume: self.volume().await,
            mute: self.mute().await,
            repeat: self.repeat().await,
            shuffle: self.shuffle().await,
        }
    }
}

pub struct PlayablesIter<'a> {
    yielded_player_ids: HashSet<PlayerId>,
    groups: GroupsIter<'a>,
    players: PlayersIter<'a>,
}

impl<'a> PlayablesIter<'a> {
    pub(super) async fn new(state: &'a State) -> Self {
        Self {
            yielded_player_ids: HashSet::default(),
            groups: state.groups().await,
            players: state.players().await,
        }
    }
}

impl<'a> Iterator for PlayablesIter<'a> {
    type Item = Playable<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(group) = self.groups.next() {
            let player = self.players.get(&group.leader_id())?;

            for player in &group.info().players {
                self.yielded_player_ids.insert(player.player_id);
            }

            return Some(Playable::from_group(group, player))
        }

        while let Some(player) = self.players.next() {
            if !self.yielded_player_ids.contains(&player.info().player_id) {
                self.yielded_player_ids.insert(player.info().player_id);
                return Some(player.into())
            }
        }

        None
    }
}