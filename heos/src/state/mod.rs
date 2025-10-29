/// Stateful management of a connected HEOS system.
///
/// Whereas the [data](crate::data) module deals with pure data at an instant in time, this module
/// contains the logic necessary to maintain a stateful connection to a HEOS system, and keep it
/// updated in real time using [events](crate::data::event) that are output from said HEOS system.
///
/// Most of the types in this module provide "views" into the state without providing ownership over
/// it. The ownership of all state belongs to the top-level [HeosConnection](crate::HeosConnection),
/// and these views can be retrieved from a [stateful](crate::Stateful) instance of that connection.

use ahash::HashMap;
use educe::Educe;
use std::hash::Hash;
use tokio::sync::{
    Mutex as AsyncMutex,
    RwLock as AsyncRwLock,
    RwLockReadGuard as AsyncRwLockReadGuard,
};
use tracing::error;

use crate::channel::Channel;
use crate::command::browse::*;
use crate::command::group::*;
use crate::command::player::*;
use crate::command::system::*;
use crate::command::CommandError;
use crate::data::event::Event;
use crate::data::group::GroupId;
use crate::data::player::PlayerId;
use crate::data::source::SourceId;
use crate::data::system::AccountStatus;
use crate::state::group::*;
use crate::state::playable::*;
use crate::state::player::*;
use crate::state::source::*;

pub mod group;
pub mod playable;
pub mod player;
pub mod source;

trait FromLockedData<'a>: Send {
    type Data;

    fn from_locked_data(
        channel: &'a AsyncMutex<Channel>,
        locked_data: AsyncRwLockReadGuard<'a, Self::Data>,
    ) -> Self;

    fn from_locked_map<K>(
        key: &K,
        channel: &'a AsyncMutex<Channel>,
        locked_map: AsyncRwLockReadGuard<'a, HashMap<K, Self::Data>>,
    ) -> Option<Self>
    where
        Self: Sized,
        K: Hash + Eq,
    {
        match AsyncRwLockReadGuard::try_map(locked_map, |map| map.get(key)) {
            Ok(locked_data) => Some(Self::from_locked_data(channel, locked_data)),
            Err(_) => None,
        }
    }
}

macro_rules! locked_data_iter {
    ($iter_name:ident, $id_type:ty, $data_type:ty, $value_type:ident) => {
        #[doc = concat!("Iterator for ", stringify!($value_type), "s")]
        pub struct $iter_name<'a> {
            channel: &'a tokio::sync::Mutex<crate::channel::Channel>,
            data: &'a tokio::sync::RwLock<ahash::HashMap<$id_type, $data_type>>,
            _guard: tokio::sync::RwLockReadGuard<'a, ahash::HashMap<$id_type, $data_type>>,
            ids: Vec<$id_type>,
        }

        impl<'a> $iter_name<'a> {
            pub(super) async fn new(
                channel: &'a tokio::sync::Mutex<crate::channel::Channel>,
                data: &'a tokio::sync::RwLock<ahash::HashMap<$id_type, $data_type>>,
            ) -> Self {
                let guard = data.read().await;
                let ids = guard.keys().cloned().collect();
                Self {
                    channel,
                    data,
                    _guard: guard,
                    ids,
                }
            }

            pub(super) fn get(&self, id: &$id_type) -> Option<$value_type<'a>> {
                let guard = self.data.try_read()
                    .expect("RwLock should already be locked with a ReadGuard");
                let data = tokio::sync::RwLockReadGuard::try_map(guard, |data| data.get(id))
                    .ok()?;
                Some($value_type::from_locked_data(self.channel, data))
            }
        }

        impl<'a> Iterator for $iter_name<'a> {
            type Item = $value_type<'a>;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                let id = self.ids.pop()?;
                self.get(&id)
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let len = self.ids.len();
                (len, Some(len))
            }
        }

        impl<'a> ExactSizeIterator for $iter_name<'a> {}
    };
}
use locked_data_iter;

/// State that is managed in a [stateful](crate::Stateful) [HeosConnection](crate::HeosConnection).
#[derive(Educe)]
#[educe(Debug)]
pub struct State {
    pub(crate) channel: AsyncMutex<Channel>,
    account: AsyncRwLock<AccountStatus>,
    #[educe(Debug(ignore))]
    sources: AsyncRwLock<HashMap<SourceId, SourceData>>,
    #[educe(Debug(ignore))]
    players: AsyncRwLock<HashMap<PlayerId, PlayerData>>,
    #[educe(Debug(ignore))]
    groups: AsyncRwLock<HashMap<GroupId, GroupData>>,
}

impl State {
    pub(crate) async fn init(channel: Channel) -> Result<Self, CommandError> {
        let mut channel = channel;
        let account = channel.send_command(CheckAccount::default()).await?;

        let state = Self {
            channel: AsyncMutex::new(channel),
            account: AsyncRwLock::new(account),
            sources: AsyncRwLock::new(HashMap::default()),
            players: AsyncRwLock::new(HashMap::default()),
            groups: AsyncRwLock::new(HashMap::default()),
        };

        state.update_sources().await?;
        state.update_players().await?;
        state.update_groups().await?;

        Ok(state)
    }

    async fn update_sources(&self) -> Result<(), CommandError> {
        let source_infos = self.channel.lock().await
            .send_command(GetSources::default()).await?;

        let mut sources = HashMap::default();
        for info in source_infos {
            let data = SourceData::get(&self.channel, info).await?;
            sources.insert(data.info.source_id, data);
        }

        *self.sources.write().await = sources;

        Ok(())
    }

    async fn update_players(&self) -> Result<(), CommandError> {
        let player_infos = self.channel.lock().await
            .send_command(GetPlayers::default()).await?;

        let mut players = HashMap::default();
        for info in player_infos {
            let data = PlayerData::get(&self.channel, info).await?;
            players.insert(data.info.player_id, data);
        }

        *self.players.write().await = players;

        Ok(())
    }

    async fn update_groups(&self) -> Result<(), CommandError> {
        let group_infos = self.channel.lock().await
            .send_command(GetGroups::default()).await?;

        let mut groups = HashMap::default();
        for info in group_infos {
            let data = GroupData::get(&self.channel, info).await?;
            groups.insert(data.info.group_id, data);
        }

        *self.groups.write().await = groups;

        Ok(())
    }

    /// Retrieve the status of the signed-in HEOS account.
    pub async fn account(&self) -> AccountStatus {
        self.account.read().await.clone()
    }

    /// Retrieve a [Source] by ID.
    ///
    /// Yields `None` if no source exists for the specified ID.
    pub async fn source(&self, source_id: &SourceId) -> Option<Source<'_>> {
        let guard = self.sources.read().await;
        Source::from_locked_map(source_id, &self.channel, guard)
    }

    /// Retrieve an iterator over all [Sources](Source).
    pub async fn sources(&self) -> SourcesIter {
        SourcesIter::new(&self.channel, &self.sources).await
    }

    /// Retrieve a [Player] by ID.
    ///
    /// Yields `None` if no player exists for the specified ID.
    pub async fn player(&self, player_id: &PlayerId) -> Option<Player<'_>> {
        let guard = self.players.read().await;
        Player::from_locked_map(player_id, &self.channel, guard)
    }

    /// Retrieve an iterator over all [Players](Player).
    pub async fn players(&self) -> PlayersIter {
        PlayersIter::new(&self.channel, &self.players).await
    }

    /// Retrieve a [Group] by ID.
    ///
    /// Yields `None` if no group exists for the specified ID.
    pub async fn group(&self, group_id: &GroupId) -> Option<Group<'_>> {
        let guard = self.groups.read().await;
        Group::from_locked_map(group_id, &self.channel, guard)
    }

    /// Retrieve an iterator over all [Groups](Group).
    pub async fn groups(&self) -> GroupsIter {
        GroupsIter::new(&self.channel, &self.groups).await
    }

    /// Retrieve a [Playable] by ID.
    ///
    /// Yields `None` if no playable exists for the specified ID.
    pub async fn playable(&self, playable_id: impl Into<PlayableId>) -> Option<Playable<'_>> {
        let playable_id = playable_id.into();
        match playable_id {
            PlayableId::Player(player_id) => self.player(&player_id).await.map(Into::into),
            PlayableId::Group(group_id) => {
                let group = self.group(&group_id).await?;
                let player = self.player(&group.leader_id()).await?;
                Some(Playable::from_group(group, player))
            }
        }
    }

    /// Retrieve an iterator over all [Playables](Playable).
    ///
    /// Note that if a player is a part of a group, it will not be yielded by this iterator, as it
    /// will be counted as "already yielded" as part of the group.
    ///
    /// As an implementation detail, this will yield all groups first, and then all players. This
    /// should NOT be relied upon, and may change without notice at any point in time.
    pub async fn playables(&self) -> PlayablesIter {
        PlayablesIter::new(self).await
    }

    pub(crate) async fn handle_event(&self, event: Event) -> Result<(), CommandError> {
        match event {
            Event::SourcesChanged => self.update_sources().await?,
            Event::PlayersChanged => self.update_players().await?,
            Event::GroupsChanged => self.update_groups().await?,
            Event::PlayerStateChanged(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    player.update_play_state(event.state).await;
                }
            },
            Event::PlayerNowPlayingChanged(event) => {
                let now_playing_info = self.channel.lock().await
                    .send_command(GetNowPlayingMedia {
                        player_id: event.player_id,
                    }).await?.value;
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    player.update_now_playing(now_playing_info).await;
                }
            },
            Event::PlayerNowPlayingProgress(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    player.update_now_playing_progress(event).await;
                }
            },
            Event::PlayerPlaybackError(event) => {
                error!(event.error);
            },
            Event::PlayerQueueChanged(event) => {
                let queue = self.channel.lock().await
                    .send_command(GetQueue {
                        player_id: event.player_id,
                        range: None,
                    }).await?;
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    *player.queue.write().await = queue;
                }
            },
            Event::PlayerVolumeChanged(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    *player.volume.write().await = event.level;
                    *player.mute.write().await = event.mute;
                }
            },
            Event::PlayerRepeatModeChanged(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    *player.repeat.write().await = event.repeat;
                }
            },
            Event::PlayerShuffleModeChanged(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    *player.shuffle.write().await = event.shuffle;
                }
            },
            Event::GroupVolumeChanged(event) => {
                if let Some(group) = self.groups.read().await.get(&event.group_id) {
                    *group.volume.write().await = event.level;
                    *group.mute.write().await = event.mute;
                }
            },
            Event::UserChanged(event) => {
                *self.account.write().await = event.account;
            },
        }
        Ok(())
    }
}