//! Abstraction around anything stateful that can play music.
//!
//! A "playable" is considered to be anything that can play music or otherwise has music "controls",
//! such as volume, next/prev, etc. A [player](super::player) is obviously a playable, but so is a
//! [group](super::group), since a group can play and control music as a group.
//!
//! For example, when a playable is a group, many aspects of the playable - such as the queue and
//! repeat/shuffle modes - are simply retrieved from the group's leader, while some aspects that a
//! group controls - such as the volume - are retrieved from the group itself.

use ahash::HashSet;
use educe::Educe;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use url::Url;

use crate::command::CommandError;
use crate::data::common::*;
use crate::data::group::*;
use crate::data::player::*;
use crate::data::queue::QueuedTrackInfo;
use crate::data::source::SourceId;
use crate::state::group::{Group, GroupsIter};
use crate::state::player::{NowPlaying, Player, PlayersIter, Queue};
use crate::state::State;

/// An ID enumeration covering all ID types of things that can be "playable".
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayableId {
    /// This playable is a player, and uses a player ID.
    Player(PlayerId),
    /// This playable is a group, and uses a group ID.
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

/// The information of a playable.
#[derive(Debug, Clone)]
pub enum PlayableInfo {
    /// This playable is a player, and uses player information.
    Player(PlayerInfo),
    /// This playable is a group, and uses group information.
    Group(GroupInfo),
}

/// A snapshot of a playable's state.
///
/// This snapshot allows full perusal of a playable's state without needing to continually await
/// asynchronous locks.
#[derive(Debug, Clone)]
pub struct PlayableSnapshot {
    /// ID enumeration of the playable.
    pub id: PlayableId,
    /// Information enumeration of the playable.
    pub info: PlayableInfo,
    /// State of the currently playing media.
    pub now_playing: NowPlaying,
    /// The queue of tracks to play next.
    pub queue: Vec<QueuedTrackInfo>,
    /// The play state.
    pub play_state: PlayState,
    /// The volume level.
    pub volume: Volume,
    /// The mute state.
    pub mute: MuteState,
    /// The repeat mode.
    pub repeat: RepeatMode,
    /// The shuffle mode.
    pub shuffle: ShuffleMode,
}

/// Live view of a group as a playable.
///
/// This combines live views of the group and the group's leader.
#[derive(Educe)]
#[educe(Deref)]
pub struct PlayableGroup<'a> {
    /// Group live view.
    #[educe(Deref)]
    pub group: Group<'a>,
    /// Leader live view.
    pub leader: Player<'a>,
}

/// Live view into a playable's state.
///
/// This provides methods to asynchronously retrieve the latest stateful data, as well as send
/// command requests relevant to this playable.
///
/// This view owns a read lock on the relevant lists of underlying states. For example, when the
/// playable is a player, it contains a read lock on the list of player states, and when the
/// playable is a group, it contains a read lock on _both_ the list of group states and player
/// states (since it keeps a live view of the group's leader as a player). This means that
/// individual playable state (including this playable) can be updated when relevant events come in,
/// but top-level change events will be delayed until this lock is released.
pub enum Playable<'a> {
    /// This playable is a player.
    Player(Player<'a>),
    /// This playable is a group.
    Group(PlayableGroup<'a>),
}

impl<'a> From<Player<'a>> for Playable<'a> {
    #[inline]
    fn from(player: Player<'a>) -> Self {
        Self::Player(player)
    }
}

macro_rules! delegate_player {
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(&self) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name(),
                Self::Group(group) => group.leader.$fn_name(),
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis async fn $fn_name:ident(&self) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v async fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name().await,
                Self::Group(group) => group.leader.$fn_name().await,
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(&self, $($param:ident: $param_t:ty),*$(,)?) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v fn $fn_name(&self, $($param: $param_t),*) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name($($param),*),
                Self::Group(group) => group.leader.$fn_name($($param),*),
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis async fn $fn_name:ident(&self, $($param:ident: $param_t:ty),*$(,)?) -> $ret_type:ty;
    ) => {
        $(#[$attr])*
        $v async fn $fn_name(&self, $($param: $param_t),*) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name($($param),*).await,
                Self::Group(group) => group.leader.$fn_name($($param),*).await,
            }
        }
    };
}

macro_rules! delegate_both {
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(&self) -> $ret_type:ty;
    ) => {
        $v fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name(),
                Self::Group(group) => group.group.$fn_name(),
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis async fn $fn_name:ident(&self) -> $ret_type:ty;
    ) => {
        $v async fn $fn_name(&self) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name().await,
                Self::Group(group) => group.group.$fn_name().await,
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis fn $fn_name:ident(&self, $($param:ident: $param_t:ty),*$(,)?) -> $ret_type:ty;
    ) => {
        $v fn $fn_name(&self, $($param: $param_t),*) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name($($param),*),
                Self::Group(group) => group.group.$fn_name($($param),*),
            }
        }
    };
    (
        $(#[$attr:meta])*
        $v:vis async fn $fn_name:ident(&self, $($param:ident: $param_t:ty),*$(,)?) -> $ret_type:ty;
    ) => {
        $v async fn $fn_name(&self, $($param: $param_t),*) -> $ret_type {
            match self {
                Self::Player(player) => player.$fn_name($($param),*).await,
                Self::Group(group) => group.group.$fn_name($($param),*).await,
            }
        }
    };
}

impl<'a> Playable<'a> {
    /// Create a playable live view from a group and it's leader.
    ///
    /// # Panics
    ///
    /// Panics if the player given as the "leader" does not have the same [PlayerId] as the group's
    /// actual leader.
    #[inline]
    pub fn from_group(group: Group<'a>, leader: Player<'a>) -> Self {
        assert_eq!(leader.info().player_id, group.leader_id());
        Self::Group(PlayableGroup {
            group,
            leader,
        })
    }

    /// Get the ID of this playable.
    pub fn id(&self) -> PlayableId {
        match self {
            Self::Player(player) => player.info().player_id.into(),
            Self::Group(group) => group.info().group_id.into(),
        }
    }

    /// Get general non-mutable information about this playable.
    pub fn info(&self) -> PlayableInfo {
        match self {
            Self::Player(player) => PlayableInfo::Player(player.info().clone()),
            Self::Group(group) => PlayableInfo::Group(group.info().clone()),
        }
    }

    delegate_player! {
        /// Retrieve the state of the currently playing media.
        pub async fn now_playing(&self) -> NowPlaying;
    }

    delegate_player! {
        /// Retrieve a view into the queue of tracks to play next.
        pub fn queue(&self) -> Queue<'_>;
    }

    delegate_player! {
        /// Retrieve the play state of this playable.
        pub async fn play_state(&self) -> PlayState;
    }

    delegate_player! {
        /// Set the play state of this player.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn set_play_state(&self, state: PlayState) -> Result<(), CommandError>;
    }

    delegate_both! {
        /// Retrieve the volume level of this playable.
        pub async fn volume(&self) -> Volume;
    }

    delegate_both! {
        /// Set the volume level of this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn set_volume(&self, level: Volume) -> Result<(), CommandError>;
    }

    delegate_both! {
        /// Increment the volume level of this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn volume_up(&self, step: Option<VolumeStep>) -> Result<(), CommandError>;
    }

    delegate_both! {
        /// Decrement the volume level of this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn volume_down(&self, step: Option<VolumeStep>) -> Result<(), CommandError>;
    }

    delegate_both! {
        /// Retrieve the mute state of this playable.
        pub async fn mute(&self) -> MuteState;
    }

    delegate_both! {
        /// Set the mute state of this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn set_mute(&self, state: MuteState) -> Result<(), CommandError>;
    }

    delegate_both! {
        /// Toggle the mute state of this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn toggle_mute(&self) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Retrieve the repeat mode of this playable.
        pub async fn repeat(&self) -> RepeatMode;
    }

    delegate_player! {
        /// Retrieve the shuffle mode of this playable.
        pub async fn shuffle(&self) -> ShuffleMode;
    }

    delegate_player! {
        /// Set the play state of this player.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn set_play_mode(
            &self,
            repeat: Option<RepeatMode>,
            shuffle: Option<ShuffleMode>,
        ) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Play the next track in this playable's queue.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn play_next(&self) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Play the previous track in this playable's queue.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn play_previous(&self) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Play a music station on this playable.
        ///
        /// See [PlayStation](crate::command::browse::PlayStation) for details on the parameters.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn play_station(
            &self,
            source_id: SourceId,
            container_id: Option<String>,
            media_id: impl Into<String>,
            name: impl Into<String>,
        ) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Play a preset on this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn play_preset(&self, preset: usize) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Play an input source on this playable.
        ///
        /// See [PlayInputSource](crate::command::browse::PlayInputSource) for details on the
        /// parameters.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn play_input_source(
            &self,
            src_player_id: Option<PlayerId>,
            input: impl Into<String>,
        ) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Play a remote stream URL on this playable.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn play_url(&self, url: impl Into<Url>) -> Result<(), CommandError>;
    }

    delegate_player! {
        /// Add media to this playable's queue.
        ///
        /// See [AddToQueue](crate::command::browse::AddToQueue) for details on the parameters.
        ///
        /// # Errors
        ///
        /// Errors if sending the underlying command errors.
        pub async fn add_to_queue(
            &self,
            source_id: SourceId,
            container_id: Option<String>,
            media_id: impl Into<String>,
            add_to_queue_type: AddToQueueType,
        ) -> Result<(), CommandError>;
    }

    /// Create a snapshot using the latest stateful data of this playable.
    ///
    /// This method will retrieve all latest stateful data and put it into a snapshot that can then
    /// be perused without further async awaiting.
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

/// Iterator over [playables](Playable).
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