use std::time::{Duration, Instant};
use educe::Educe;
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
use crate::data::event::PlayerNowPlayingProgress;
use crate::state::{locked_data_iter, FromLockedData};

#[derive(Educe, Clone)]
#[educe(Debug)]
pub struct NowPlayingProgress {
    pub elapsed: Duration,
    pub duration: Duration,
    #[educe(Debug(ignore))]
    baseline: Option<Instant>,
}

impl NowPlayingProgress {
    pub fn interpolated_elapsed(&self) -> Duration {
        if let Some(baseline) = self.baseline {
            let diff = Instant::now() - baseline;
            (self.elapsed + diff).min(self.duration)
        } else {
            self.elapsed
        }
    }
}

#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub info: NowPlayingInfo,
    pub progress: NowPlayingProgress,
}

#[derive(Debug)]
pub(super) struct PlayerData {
    pub info: PlayerInfo,
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
            progress: NowPlayingProgress {
                elapsed: Duration::default(),
                duration: Duration::default(),
                baseline: None,
            }
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

    pub async fn update_play_state(&self, new_play_state: PlayState) {
        let mut play_state = self.play_state.write().await;
        let mut now_playing = self.now_playing.write().await;

        *play_state = new_play_state;
        match new_play_state {
            PlayState::Play => {
                if now_playing.progress.baseline.is_none() {
                    now_playing.progress.baseline = Some(Instant::now());
                }
            },
            PlayState::Pause | PlayState::Stop => {
                now_playing.progress.elapsed = now_playing.progress.interpolated_elapsed();
                now_playing.progress.baseline = None;
            },
        }
    }

    pub async fn update_now_playing(&self, info: NowPlayingInfo) {
        let play_state = self.play_state.read().await;
        let mut now_playing = self.now_playing.write().await;

        let baseline = if *play_state == PlayState::Play {
            Some(Instant::now())
        } else {
            None
        };

        *now_playing = NowPlaying {
            info,
            progress: NowPlayingProgress {
                elapsed: Duration::default(),
                duration: Duration::default(),
                baseline,
            }
        }
    }

    pub async fn update_now_playing_progress(&self, event: PlayerNowPlayingProgress) {
        let play_state = self.play_state.read().await;
        let mut now_playing = self.now_playing.write().await;
        now_playing.progress.elapsed = event.elapsed;
        now_playing.progress.duration = event.duration;
        if *play_state == PlayState::Play {
            // update the baseline
            now_playing.progress.baseline = Some(Instant::now());
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub info: PlayerInfo,
    pub now_playing: NowPlaying,
    pub queue: Vec<SongInfo>,
    pub play_state: PlayState,
    pub volume: Volume,
    pub mute: MuteState,
    pub repeat: RepeatMode,
    pub shuffle: ShuffleMode,
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
    ) -> Self {
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
        self.channel.lock().await
            .send_command(SetPlayState {
                player_id: self.data.info.player_id,
                state,
            }).await
    }

    pub async fn volume(&self) -> Volume {
        *self.data.volume.read().await
    }

    pub async fn set_volume(&self, level: Volume) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetVolume {
                player_id: self.data.info.player_id,
                level,
            }).await
    }

    pub async fn volume_up(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(VolumeUp {
                player_id: self.data.info.player_id,
                step,
            }).await
    }

    pub async fn volume_down(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(VolumeDown {
                player_id: self.data.info.player_id,
                step,
            }).await
    }

    pub async fn mute(&self) -> MuteState {
        *self.data.mute.read().await
    }

    pub async fn set_mute(&self, state: MuteState) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetMute {
                player_id: self.data.info.player_id,
                state,
            }).await
    }

    pub async fn toggle_mute(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(ToggleMute {
                player_id: self.data.info.player_id,
            }).await
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
        self.channel.lock().await
            .send_command(SetPlayMode {
                player_id: self.data.info.player_id,
                repeat,
                shuffle,
            }).await
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

    pub async fn snapshot(&self) -> PlayerSnapshot {
        PlayerSnapshot {
            info: self.data.info.clone(),
            now_playing: self.data.now_playing.read().await.clone(),
            queue: self.data.queue.read().await.clone(),
            play_state: self.data.play_state.read().await.clone(),
            volume: self.data.volume.read().await.clone(),
            mute: self.data.mute.read().await.clone(),
            repeat: self.data.repeat.read().await.clone(),
            shuffle: self.data.shuffle.read().await.clone(),
        }
    }
}

locked_data_iter!(PlayersIter, PlayerId, PlayerData, Player);

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
        let queue = self.queue.read().await;
        if let Some(song) = queue.get(idx) {
            self.player.channel.lock().await
                .send_command(PlayQueueItem {
                    player_id: self.player.data.info.player_id,
                    queue_id: song.queue_id,
                }).await
        } else {
            Err(CommandError::ParamOutOfRange)
        }
    }

    pub async fn remove(&mut self, idx: usize) -> Result<(), CommandError> {
        let queue = self.queue.read().await;
        if let Some(song) = queue.get(idx) {
            self.player.channel.lock().await
                .send_command(RemoveFromQueue {
                    player_id: self.player.data.info.player_id,
                    queue_ids: vec![song.queue_id],
                }).await
        } else {
            Err(CommandError::ParamOutOfRange)
        }
    }

    pub async fn clear(&mut self) -> Result<(), CommandError> {
        self.player.channel.lock().await
            .send_command(ClearQueue {
                player_id: self.player.data.info.player_id,
            }).await
    }
}