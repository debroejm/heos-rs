use std::time::Duration;
use tokio::sync::{
    Mutex as AsyncMutex,
    RwLock as AsyncRwLock,
    RwLockReadGuard as AsyncRwLockReadGuard,
};
use url::Url;
use crate::command::browse::*;
use crate::command::player::*;
use crate::command::CommandError;
use crate::data::common::*;
use crate::data::player::*;
use crate::data::song::*;
use crate::data::source::SourceId;
use crate::channel::Channel;
use crate::state::FromLockedData;

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub info: NowPlayingInfo,
    pub elapsed: Duration,
    pub duration: Duration,
}

#[derive(Debug)]
pub(super) struct PlayerData {
    pub info: PlayerInfo,
    // TODO: separate into individual locks
    pub now_playing: AsyncRwLock<NowPlaying>,
    pub queue: AsyncRwLock<Vec<SongInfo>>,
    pub play_state: AsyncRwLock<PlayState>,
    pub volume: AsyncRwLock<Volume>,
    pub mute: AsyncRwLock<MuteState>,
    pub repeat: AsyncRwLock<RepeatMode>,
    pub shuffle: AsyncRwLock<ShuffleMode>,
}

impl PlayerData {
    pub async fn get(channel: &AsyncMutex<Channel>, info: PlayerInfo) -> Result<Self, CommandError> {
        let now_playing_info = channel.lock().await
            .send_command(GetNowPlayingMedia {
                player_id: info.player_id,
            }).await?.value;
        let now_playing = NowPlaying {
            info: now_playing_info,
            elapsed: Duration::default(),
            duration: Duration::default(),
        };
        let queue = channel.lock().await
            .send_command(GetQueue {
                player_id: info.player_id,
                range: None,
            }).await?;
        let play_state = channel.lock().await
            .send_command(GetPlayState {
                player_id: info.player_id,
            }).await?;
        let volume = channel.lock().await
            .send_command(GetVolume {
                player_id: info.player_id,
            }).await?;
        let mute = channel.lock().await
            .send_command(GetMute {
                player_id: info.player_id,
            }).await?;
        let play_mode = channel.lock().await
            .send_command(GetPlayMode {
                player_id: info.player_id,
            }).await?;

        Ok(Self {
            info,
            now_playing: AsyncRwLock::new(now_playing),
            queue: AsyncRwLock::new(queue),
            play_state: AsyncRwLock::new(play_state),
            volume: AsyncRwLock::new(volume),
            mute: AsyncRwLock::new(mute),
            repeat: AsyncRwLock::new(play_mode.repeat),
            shuffle: AsyncRwLock::new(play_mode.shuffle),
        })
    }
}

pub struct Player<'a> {
    channel: &'a AsyncMutex<Channel>,
    data: AsyncRwLockReadGuard<'a, PlayerData>,
}

impl<'a> FromLockedData<'a> for Player<'a> {
    type Data = PlayerData;

    #[inline]
    fn from_locked_data(
        channel: &'a AsyncMutex<Channel>,
        data: AsyncRwLockReadGuard<'a, PlayerData>,
    ) -> Self
    where
        Self: 'a,
    {
        Self {
            channel,
            data,
        }
    }
}

impl<'a> Player<'a> {
    #[inline]
    pub fn info(&self) -> &PlayerInfo {
        &self.data.info
    }

    pub async fn now_playing(&self) -> NowPlaying {
        self.data.now_playing.read().await.clone()
    }

    #[inline]
    pub fn queue(&self) -> Queue<'_> {
        Queue {
            player: self,
            queue: &self.data.queue,
        }
    }

    pub async fn play_state(&self) -> PlayState {
        *self.data.play_state.read().await
    }

    pub async fn set_play_state(&self, state: PlayState) -> Result<(), CommandError> {
        let mut play_state = self.data.play_state.write().await;
        self.channel.lock().await
            .send_command(SetPlayState {
                player_id: self.data.info.player_id,
                state,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local play_state value
        *play_state = state;
        Ok(())
    }

    pub async fn volume(&self) -> Volume {
        *self.data.volume.read().await
    }

    pub async fn set_volume(&self, level: Volume) -> Result<(), CommandError> {
        let mut volume = self.data.volume.write().await;
        self.channel.lock().await
            .send_command(SetVolume {
                player_id: self.data.info.player_id,
                level,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local volume value
        *volume = level;
        Ok(())
    }

    pub async fn volume_up(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        let mut volume = self.data.volume.write().await;
        self.channel.lock().await
            .send_command(VolumeUp {
                player_id: self.data.info.player_id,
                step,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local volume value
        *volume = volume.saturating_add(step.unwrap_or_default());
        Ok(())
    }

    pub async fn volume_down(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        let mut volume = self.data.volume.write().await;
        self.channel.lock().await
            .send_command(VolumeDown {
                player_id: self.data.info.player_id,
                step,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local volume value
        *volume = volume.saturating_sub(step.unwrap_or_default());
        Ok(())
    }

    pub async fn mute(&self) -> MuteState {
        *self.data.mute.read().await
    }

    pub async fn set_mute(&self, state: MuteState) -> Result<(), CommandError> {
        let mut mute = self.data.mute.write().await;
        self.channel.lock().await
            .send_command(SetMute {
                player_id: self.data.info.player_id,
                state,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local mute value
        *mute = state;
        Ok(())
    }

    pub async fn toggle_mute(&self) -> Result<(), CommandError> {
        let mut mute = self.data.mute.write().await;
        self.channel.lock().await
            .send_command(ToggleMute {
                player_id: self.data.info.player_id,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local mute value
        *mute = match *mute {
            MuteState::On => MuteState::Off,
            MuteState::Off => MuteState::On,
        };
        Ok(())
    }

    pub async fn repeat(&self) -> RepeatMode {
        *self.data.repeat.read().await
    }

    pub async fn shuffle(&self) -> ShuffleMode {
        *self.data.shuffle.read().await
    }

    pub async fn set_play_mode(
        &self,
        repeat: Option<RepeatMode>,
        shuffle: Option<ShuffleMode>,
    ) -> Result<(), CommandError> {
        let repeat_lock = match repeat {
            Some(_) => Some(self.data.repeat.write().await),
            None => None,
        };
        let shuffle_lock = match shuffle {
            Some(_) => Some(self.data.shuffle.write().await),
            None => None,
        };
        self.channel.lock().await
            .send_command(SetPlayMode {
                player_id: self.data.info.player_id,
                repeat,
                shuffle,
            }).await?;
        // Assuming the above completes successfully, pre-emptively update the local repeat/shuffle values
        if let Some(repeat) = repeat {
            *repeat_lock.unwrap() = repeat;
        }
        if let Some(shuffle) = shuffle {
            *shuffle_lock.unwrap() = shuffle;
        }
        Ok(())
    }

    pub async fn play_next(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayNext {
                player_id: self.data.info.player_id,
            }).await
    }

    pub async fn play_previous(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayPrevious {
                player_id: self.data.info.player_id,
            }).await
    }

    pub async fn play_station(
        &self,
        source_id: SourceId,
        container_id: Option<String>,
        media_id: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayStation {
                player_id: self.data.info.player_id,
                source_id,
                container_id,
                media_id: media_id.into(),
                name: name.into(),
            }).await
    }

    pub async fn play_preset(&self, preset: usize) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayPreset {
                player_id: self.data.info.player_id,
                preset,
            }).await
    }

    pub async fn play_input_source(
        &self,
        src_player_id: Option<PlayerId>,
        input: impl Into<String>,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayInputSource {
                player_id: self.data.info.player_id,
                src_player_id,
                input: input.into(),
            }).await
    }

    pub async fn play_url(
        &self,
        url: impl Into<Url>,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayUrl {
                player_id: self.data.info.player_id,
                url: url.into()
            }).await
    }

    pub async fn add_to_queue(
        &self,
        source_id: SourceId,
        container_id: Option<String>,
        media_id: impl Into<String>,
        add_to_queue_type: AddToQueueType,
    ) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(AddToQueue {
                player_id: self.data.info.player_id,
                source_id,
                container_id,
                media_id: media_id.into(),
                add_to_queue_type,
            }).await
    }
}

pub struct Queue<'a> {
    player: &'a Player<'a>,
    queue: &'a AsyncRwLock<Vec<SongInfo>>,
}

impl<'a> Queue<'a> {
    pub async fn data(&self) -> AsyncRwLockReadGuard<'a, Vec<SongInfo>> {
        self.queue.read().await
    }

    pub async fn save(&mut self, name: impl Into<String>) -> Result<(), CommandError> {
        self.player.channel.lock().await
            .send_command(SaveQueue {
                player_id: self.player.data.info.player_id,
                name: name.into(),
            }).await
    }

    pub async fn play(&mut self, idx: usize) -> Result<(), CommandError> {
        let queue = self.queue.write().await;
        if let Some(song) = queue.get(idx) {
            self.player.channel.lock().await
                .send_command(PlayQueueItem {
                    player_id: self.player.data.info.player_id,
                    queue_id: song.queue_id,
                }).await?;
            // TODO: How does this change the local queue?
            Ok(())
        } else {
            Err(CommandError::ParamOutOfRange)
        }
    }

    pub async fn remove(&mut self, idx: usize) -> Result<(), CommandError> {
        let mut queue = self.queue.write().await;
        if let Some(song) = queue.get(idx) {
            self.player.channel.lock().await
                .send_command(RemoveFromQueue {
                    player_id: self.player.data.info.player_id,
                    queue_ids: vec![song.queue_id],
                }).await?;
            queue.remove(idx);
            Ok(())
        } else {
            Err(CommandError::ParamOutOfRange)
        }
    }

    pub async fn clear(&mut self) -> Result<(), CommandError> {
        let mut queue = self.queue.write().await;
        self.player.channel.lock().await
            .send_command(ClearQueue {
                player_id: self.player.data.info.player_id,
            }).await?;
        queue.clear();
        Ok(())
    }
}