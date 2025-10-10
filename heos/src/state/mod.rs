use std::hash::Hash;
use ahash::HashMap;
use std::time::Duration;
use educe::Educe;
use tracing::error;
use tokio::sync::{
    Mutex as AsyncMutex,
    RwLock as AsyncRwLock,
    RwLockReadGuard as AsyncRwLockReadGuard,
};

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
use crate::channel::Channel;
use crate::state::group::*;
use crate::state::player::*;
use crate::state::source::*;

pub mod group;
pub mod player;
pub mod source;

trait FromLockedData<'a> {
    type Data;

    fn from_locked_data(
        channel: &'a AsyncMutex<Channel>,
        locked_data: AsyncRwLockReadGuard<'a, Self::Data>,
    ) -> Self
    where
        Self: 'a;

    fn from_locked_map<K>(
        key: &K,
        channel: &'a AsyncMutex<Channel>,
        locked_map: AsyncRwLockReadGuard<'a, HashMap<K, Self::Data>>,
    ) -> Option<Self>
    where
        Self: Sized + 'a,
        K: Hash + Eq,
    {
        match AsyncRwLockReadGuard::try_map(locked_map, |map| map.get(key)) {
            Ok(locked_data) => Some(Self::from_locked_data(channel, locked_data)),
            Err(_) => None,
        }
    }
}

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
    pub async fn init(channel: Channel) -> Result<Self, CommandError> {
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

    pub async fn account(&self) -> AccountStatus {
        self.account.read().await.clone()
    }

    pub async fn source(&self, source_id: &SourceId) -> Option<Source<'_>> {
        let guard = self.sources.read().await;
        Source::from_locked_map(source_id, &self.channel, guard)
    }

    pub async fn sources(&self) -> LockedDataIter<SourceId, Source> {
        LockedDataIter::new(&self.channel, &self.sources).await
    }

    pub async fn player(&self, player_id: &PlayerId) -> Option<Player<'_>> {
        let guard = self.players.read().await;
        Player::from_locked_map(player_id, &self.channel, guard)
    }

    pub async fn players(&self) -> LockedDataIter<PlayerId, Player> {
        LockedDataIter::new(&self.channel, &self.players).await
    }

    pub async fn group(&self, group_id: &GroupId) -> Option<Group<'_>> {
        let guard = self.groups.read().await;
        Group::from_locked_map(group_id, &self.channel, guard)
    }

    pub async fn groups(&self) -> LockedDataIter<GroupId, Group> {
        LockedDataIter::new(&self.channel, &self.groups).await
    }

    pub(crate) async fn handle_event(&self, event: Event) -> Result<(), CommandError> {
        match event {
            Event::SourcesChanged => self.update_sources().await?,
            Event::PlayersChanged => self.update_players().await?,
            Event::GroupsChanged => self.update_groups().await?,
            Event::PlayerStateChanged(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    *player.play_state.write().await = event.state;
                }
            },
            Event::PlayerNowPlayingChanged(event) => {
                let now_playing_info = self.channel.lock().await
                    .send_command(GetNowPlayingMedia {
                        player_id: event.player_id,
                    }).await?.value;
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    *player.now_playing.write().await = NowPlaying {
                        info: now_playing_info,
                        elapsed: Duration::default(),
                        duration: Duration::default(),
                    };
                }
            },
            Event::PlayerNowPlayingProgress(event) => {
                if let Some(player) = self.players.read().await.get(&event.player_id) {
                    let mut now_playing = player.now_playing.write().await;
                    now_playing.elapsed = event.elapsed;
                    now_playing.duration = event.duration;
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

#[allow(private_bounds)]
pub struct LockedDataIter<'a, K, V: FromLockedData<'a>> {
    channel: &'a AsyncMutex<Channel>,
    data: &'a AsyncRwLock<HashMap<K, V::Data>>,
    _guard: AsyncRwLockReadGuard<'a, HashMap<K, V::Data>>,
    keys: Vec<K>,
}

#[allow(private_bounds)]
impl<'a, K, V: FromLockedData<'a>> LockedDataIter<'a, K, V> {
    async fn new(channel: &'a AsyncMutex<Channel>, data: &'a AsyncRwLock<HashMap<K, V::Data>>) -> Self
    where
        K: Clone,
    {
        let guard = data.read().await;
        let keys = guard.keys().map(Clone::clone).collect();
        Self {
            channel,
            data,
            _guard: guard,
            keys,
        }
    }
}

impl<'a, K, V> Iterator for LockedDataIter<'a, K, V>
where
    K: Hash + Eq,
    V: FromLockedData<'a> + 'a,
{
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.keys.pop()?;
        let guard = self.data.try_read()
            .expect("RwLock should already be locked with a ReadGuard");
        let data = AsyncRwLockReadGuard::map(guard, |data| &data[&key]);
        let value = V::from_locked_data(self.channel, data);
        Some(value)
    }
}