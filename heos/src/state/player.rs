/// Stateful player management.

use educe::Educe;
use std::time::{Duration, Instant};
use tokio::sync::{
    Mutex as AsyncMutex,
    RwLock as AsyncRwLock,
    RwLockReadGuard as AsyncRwLockReadGuard,
};
use url::Url;

use crate::channel::Channel;
use crate::command::browse::*;
use crate::command::player::*;
use crate::command::CommandError;
use crate::data::common::*;
use crate::data::event::PlayerNowPlayingProgress;
use crate::data::player::*;
use crate::data::song::*;
use crate::data::source::SourceId;
use crate::state::{locked_data_iter, FromLockedData};

/// The progress of the currently playing music.
#[derive(Educe, Clone)]
#[educe(Debug)]
pub struct NowPlayingProgress {
    /// How much of the track has elapsed.
    ///
    /// This value is based on the last received
    /// [PlayerNowPlayingProgress](crate::data::event::Event::PlayerNowPlayingProgress) event, and
    /// is not accurate to more than a couple of seconds.
    ///
    /// To retrieve a more accurate value, you can use [NowPlayingProgress::interpolated_elapsed()]
    /// to get a value with an added offset based on the last time an event was received.
    pub elapsed: Duration,
    /// Total duration of the current track.
    pub duration: Duration,
    #[educe(Debug(ignore))]
    baseline: Option<Instant>,
}

impl NowPlayingProgress {
    /// Calculate an interpolated elapsed time based on the last received
    /// [PlayerNowPlayingProgress](crate::data::event::Event::PlayerNowPlayingProgress) event.
    ///
    /// The result is calculated by adding the amount of time that has passed since the last event
    /// to the last known elapsed value. Note that if the currently playing track is paused or
    /// stopped, this method will no-op and just yield the last calculated `elapsed` value in order
    /// to prevent the perceived elapsed time from continuing to grow while the track is not
    /// actually playing.
    pub fn interpolated_elapsed(&self) -> Duration {
        if let Some(baseline) = self.baseline {
            let diff = Instant::now() - baseline;
            (self.elapsed + diff).min(self.duration)
        } else {
            self.elapsed
        }
    }
}

/// The state of the currently playing media.
#[derive(Debug, Clone)]
pub struct NowPlaying {
    /// Information about the currently playing track.
    pub info: NowPlayingInfo,
    /// Progress of the currently playing track.
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

/// A snapshot of a player's state.
///
/// This snapshot allows full perusal of a player's state without needing to continually await
/// asynchronous locks.
#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    /// Non-mutable information about the player.
    pub info: PlayerInfo,
    /// State of the currently playing media.
    pub now_playing: NowPlaying,
    /// The queue of tracks to play next.
    pub queue: Vec<SongInfo>,
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

/// Live view into a player's state.
///
/// This provides methods to asynchronously retrieve the latest stateful data, as well as send
/// command requests relevant to this player.
///
/// This view owns a read lock on the list of player states. This means that individual player state
/// (including this player) can be updated when relevant events come in, but
/// [PlayersChanged](crate::data::event::Event::PlayersChanged) events will be delayed until this
/// lock is released.
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
    /// Get general non-mutable information about this player.
    #[inline]
    pub fn info(&self) -> &PlayerInfo {
        &self.data.info
    }

    /// Retrieve the state of the currently playing media.
    pub async fn now_playing(&self) -> NowPlaying {
        self.data.now_playing.read().await.clone()
    }

    /// Retrieve a view into the queue of tracks to play next.
    #[inline]
    pub fn queue(&self) -> Queue<'_> {
        Queue {
            player: self,
            queue: &self.data.queue,
        }
    }

    /// Retrieve the play state of this player.
    pub async fn play_state(&self) -> PlayState {
        *self.data.play_state.read().await
    }

    /// Set the play state of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetPlayState] command errors.
    pub async fn set_play_state(&self, state: PlayState) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetPlayState {
                player_id: self.data.info.player_id,
                state,
            }).await
    }

    /// Retrieve the volume level of this player.
    pub async fn volume(&self) -> Volume {
        *self.data.volume.read().await
    }

    /// Set the volume level of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetVolume] command errors.
    pub async fn set_volume(&self, level: Volume) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetVolume {
                player_id: self.data.info.player_id,
                level,
            }).await
    }

    /// Increment the volume level of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [VolumeUp] command errors.
    pub async fn volume_up(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(VolumeUp {
                player_id: self.data.info.player_id,
                step,
            }).await
    }

    /// Decrement the volume level of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [VolumeDown] command errors.
    pub async fn volume_down(&self, step: Option<VolumeStep>) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(VolumeDown {
                player_id: self.data.info.player_id,
                step,
            }).await
    }

    /// Retrieve the mute state of this player.
    pub async fn mute(&self) -> MuteState {
        *self.data.mute.read().await
    }

    /// Set the mute state of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetMute] command errors.
    pub async fn set_mute(&self, state: MuteState) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(SetMute {
                player_id: self.data.info.player_id,
                state,
            }).await
    }

    /// Toggle the mute state of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [ToggleMute] command errors.
    pub async fn toggle_mute(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(ToggleMute {
                player_id: self.data.info.player_id,
            }).await
    }

    /// Retrieve the repeat mode of this player.
    pub async fn repeat(&self) -> RepeatMode {
        *self.data.repeat.read().await
    }

    /// Retrieve the shuffle mode of this player.
    pub async fn shuffle(&self) -> ShuffleMode {
        *self.data.shuffle.read().await
    }

    /// Set the repeat mode and/or shuffle mode of this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SetPlayMode] command errors.
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

    /// Play the next track in this player's queue.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayNext] command errors.
    pub async fn play_next(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayNext {
                player_id: self.data.info.player_id,
            }).await
    }

    /// Play the track that played previously from this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayPrevious] command errors.
    pub async fn play_previous(&self) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayPrevious {
                player_id: self.data.info.player_id,
            }).await
    }

    /// Play a music station on this player.
    ///
    /// See [PlayStation] for details on the parameters.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayStation] command errors.
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

    /// Play a preset on this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayPreset] command errors.
    pub async fn play_preset(&self, preset: usize) -> Result<(), CommandError> {
        self.channel.lock().await
            .send_command(PlayPreset {
                player_id: self.data.info.player_id,
                preset,
            }).await
    }

    /// Play an input source on this player.
    ///
    /// See [PlayInputSource] for details on the parameters.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayInputSource] command errors.
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

    /// Play a remote stream URL on this player.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayUrl] command errors.
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

    /// Add media to this player's queue.
    ///
    /// See [AddToQueue] for details on the parameters.
    ///
    /// # Errors
    ///
    /// Errors if sending an [AddToQueue] command errors.
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

    /// Create a snapshot using the latest stateful data of this player.
    ///
    /// This method will retrieve all latest stateful data and put it into a snapshot that can then
    /// be perused without further async awaiting.
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

/// Live view into a player's queue.
///
/// This provides methods to asynchronously manipulate a player's queue.
pub struct Queue<'a> {
    player: &'a Player<'a>,
    queue: &'a AsyncRwLock<Vec<SongInfo>>,
}

impl<'a> Queue<'a> {
    /// Get a read lock on the queues data so it can be inspected.
    pub async fn data(&self) -> AsyncRwLockReadGuard<'a, Vec<SongInfo>> {
        self.queue.read().await
    }

    /// Save the current queue as a HEOS playlist.
    ///
    /// # Errors
    ///
    /// Errors if sending a [SaveQueue] command errors.
    pub async fn save(&mut self, name: impl Into<String>) -> Result<(), CommandError> {
        self.player.channel.lock().await
            .send_command(SaveQueue {
                player_id: self.player.data.info.player_id,
                name: name.into(),
            }).await
    }

    /// Play the track at the given queue index.
    ///
    /// This will replace the currently playing track with the one specified, if successful.
    ///
    /// # Errors
    ///
    /// Errors if sending a [PlayQueueItem] command errors.
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

    /// Remove the track at the given queue index.
    ///
    /// # Errors
    ///
    /// Errors if sending a [RemoveFromQueue] command errors.
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

    /// Clear the queue.
    ///
    /// # Errors
    ///
    /// Errors if sending a [ClearQueue] command errors.
    pub async fn clear(&mut self) -> Result<(), CommandError> {
        self.player.channel.lock().await
            .send_command(ClearQueue {
                player_id: self.player.data.info.player_id,
            }).await
    }
}