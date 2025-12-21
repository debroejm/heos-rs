use egui_async::bind::MaybeSend;
use egui_async::Bind;
use heos::command::browse::{AddToQueue, PlayInputSource, PlayStation};
use heos::command::group::SetGroup;
use heos::command::player::{MoveQueue, RemoveFromQueue};
use heos::command::{CommandError, CommandErrorCode};
use heos::data::common::Volume;
use heos::data::group::GroupRole;
use heos::data::media::{HeosService, MediaContainerBase, MediaItem, MediaItemBase};
use heos::data::player::{AddToQueueType, PlayState, PlayerId, RepeatMode, ShuffleMode};
use heos::data::queue::QueueId;
use heos::data::source::SourceId;
use heos::state::playable::{Playable, PlayableId, PlayableInfo};
use heos::{HeosConnection, Stateful};
use std::iter;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::screen::media_bar::ControlButton;

pub struct Actions {
    heos: Arc<HeosConnection<Stateful>>,
    binds: Vec<Bind<(), CommandError>>,
}

impl Actions {
    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        Self {
            heos,
            binds: vec![],
        }
    }

    pub fn check_binds(&mut self) -> bool {
        self.binds.retain_mut(|bind| {
            if let Some(_) = bind.take() {
                // TODO create a popup if there is an error
                false
            } else {
                true
            }
        });
        self.binds.is_empty()
    }

    fn add_bind(&mut self, fut: impl Future<Output = Result<(), CommandError>> + MaybeSend + 'static) {
        let mut bind = Bind::new(true);
        bind.request(fut);
        self.binds.push(bind);
    }

    async fn try_playable<'a>(
        heos: &'a Arc<HeosConnection<Stateful>>,
        playable_id: PlayableId,
    ) -> Result<Playable<'a>, CommandError> {
        match heos.playable(playable_id).await {
            Some(playable) => Ok(playable),
            None => {
                warn!(%playable_id, error = "no player found for ID");
                Err(CommandError::Failure {
                    code: CommandErrorCode::InvalidId,
                    text: format!("No playable found for ID ({})", playable_id),
                })
            }
        }
    }

    pub fn media_control(&mut self, playable_id: PlayableId, button_type: ControlButton) {
        debug!(?playable_id, ?button_type, "Media control activated");
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            match button_type {
                ControlButton::PlayPause => {
                    playable.set_play_state(match playable.play_state().await {
                        PlayState::Stop | PlayState::Pause => PlayState::Play,
                        PlayState::Play => PlayState::Pause,
                    }).await
                },
                ControlButton::Next => playable.play_next().await,
                ControlButton::Prev => playable.play_previous().await,
                ControlButton::Repeat => playable.set_play_mode(
                    Some(match playable.repeat().await {
                        RepeatMode::Off => RepeatMode::One,
                        RepeatMode::One => RepeatMode::All,
                        RepeatMode::All => RepeatMode::Off,
                    }),
                    None,
                ).await,
                ControlButton::Shuffle => playable.set_play_mode(
                    None,
                    Some(match playable.shuffle().await {
                        ShuffleMode::Off => ShuffleMode::On,
                        ShuffleMode::On => ShuffleMode::Off,
                    }),
                ).await,
            }
        });
    }

    pub fn toggle_mute(&mut self, playable_id: PlayableId) {
        debug!(?playable_id, "Toggling mute");
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            playable.toggle_mute().await
        });
    }

    pub fn set_volume(&mut self, playable_id: PlayableId, volume: Volume) {
        debug!(?playable_id, ?volume, "Setting volume");
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            playable.set_volume(volume).await
        });
    }

    async fn remove_from_any_group_impl(
        heos: &Arc<HeosConnection<Stateful>>,
        player_id: PlayerId,
    ) -> Result<(), CommandError> {
        for group in heos.groups().await {
            if let Some(group_player) = group.info().player(player_id) {
                if group_player.role == GroupRole::Leader {
                    let player_ids = group.info().players.iter()
                        .filter(|player| player.role != GroupRole::Leader)
                        .map(|player| player.player_id)
                        .collect::<Vec<_>>();
                    // Delete the group first
                    heos.command(SetGroup {
                        player_ids: vec![group_player.player_id],
                    }).await?;
                    // Now recreate the group with the remaining players, if necessary
                    if player_ids.len() > 1 {
                        heos.command(SetGroup {
                            player_ids,
                        }).await?;
                    }
                } else {
                    let player_ids = iter::once(group.leader_id())
                        .chain(group.info().players.iter()
                            .filter(|player| player.role != GroupRole::Leader && player.player_id != player_id)
                            .map(|player| player.player_id))
                        .collect::<Vec<_>>();
                    heos.command(SetGroup {
                        player_ids,
                    }).await?;
                }
            }
        }
        Ok(())
    }

    pub fn add_to_group(&mut self, playable_id: PlayableId, new_player_id: PlayerId) {
        debug!(target_id = ?playable_id, ?new_player_id, "Adding player to group");
        let heos = self.heos.clone();
        self.add_bind(async move {
            // Remove from any existing groups first
            Self::remove_from_any_group_impl(&heos, new_player_id).await?;

            let Some(playable) = heos.playable(playable_id).await else {
                return Ok(())
            };

            match playable {
                Playable::Group(group) => {
                    if group.group.info().player(new_player_id).is_some() {
                        // Already in the group
                        return Ok(())
                    }
                    let player_ids = iter::once(group.leader_id())
                        .chain(group.group.info().players.iter()
                            .filter(|player| player.role != GroupRole::Leader)
                            .map(|player| player.player_id))
                        .chain(iter::once(new_player_id))
                        .collect::<Vec<_>>();
                    heos.command(SetGroup {
                        player_ids,
                    }).await?;
                },
                Playable::Player(player) => {
                    heos.command(SetGroup {
                        player_ids: vec![player.info().player_id, new_player_id],
                    }).await?;
                },
            }

            Ok(())
        });
    }

    pub fn remove_from_any_group(&mut self, player_id: PlayerId) {
        debug!(?player_id, "Removing player from any groups");
        let heos = self.heos.clone();
        self.add_bind(async move {
            Self::remove_from_any_group_impl(&heos, player_id).await
        });
    }

    pub fn move_queue(&mut self, playable_id: PlayableId, from: QueueId, to: QueueId) {
        debug!(?playable_id, ?from, ?to, "Moving queued track");
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            let player_id = match playable.info() {
                PlayableInfo::Group(group) => group.leader().player_id,
                PlayableInfo::Player(player) => player.player_id,
            };
            heos.command(MoveQueue {
                player_id,
                src_queue_ids: vec![from],
                dst_queue_id: to,
            }).await
        });
    }

    pub fn remove_from_queue(&mut self, playable_id: PlayableId, queue_id: QueueId) {
        debug!(?playable_id, ?queue_id, "Removing queued track");
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            let player_id = match playable.info() {
                PlayableInfo::Group(group) => group.leader().player_id,
                PlayableInfo::Player(player) => player.player_id,
            };
            heos.command(RemoveFromQueue {
                player_id,
                queue_ids: vec![queue_id],
            }).await
        });
    }

    pub fn play_station(
        &mut self,
        playable_id: PlayableId,
        source_id: SourceId,
        parent_id: Option<String>,
        item: MediaItem,
    ) {
        debug!(
            ?playable_id,
            ?source_id,
            ?parent_id,
            ?item,
            "Playing station",
        );
        let container_id = match item.try_as_media_container_ref() {
            Some(container) => Some(container.container_id().to_string()),
            None => parent_id,
        };
        let media_id = match item.media_id() {
            Some(mid) => mid.to_string(),
            None => {
                warn!(?item, "Tried to play station with no 'media_id'");
                return
            }
        };
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            let player_id = match playable.info() {
                PlayableInfo::Group(group) => group.leader().player_id,
                PlayableInfo::Player(player) => player.player_id,
            };
            heos.command(PlayStation {
                player_id,
                source_id,
                container_id,
                media_id,
                name: item.name().to_string(),
            }).await
        });
    }

    pub fn play_input_source(
        &mut self,
        playable_id: PlayableId,
        source_id: SourceId,
        item: MediaItem,
    ) {
        debug!(
            ?playable_id,
            ?source_id,
            ?item,
            "Playing input source",
        );
        let heos_service = match HeosService::try_from(item) {
            Ok(hs) => hs,
            Err(error) => {
                warn!(?error, "Tried to play input source with mismatched MediaItem type");
                return
            }
        };
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            let player_id = match playable.info() {
                PlayableInfo::Group(group) => group.leader().player_id,
                PlayableInfo::Player(player) => player.player_id,
            };

            let src_player_id = if player_id == heos_service.src_player_id {
                None
            } else {
                Some(heos_service.src_player_id)
            };

            heos.command(PlayInputSource {
                player_id,
                src_player_id,
                input: heos_service.name,
            }).await
        });
    }

    pub fn add_to_queue(
        &mut self,
        playable_id: PlayableId,
        source_id: SourceId,
        parent_id: Option<String>,
        item: MediaItem,
        add_to_queue_type: AddToQueueType,
    ) {
        debug!(
            ?playable_id,
            ?source_id,
            ?parent_id,
            ?item,
            ?add_to_queue_type,
            "Adding item to queue",
        );
        let container_id = match item.try_as_media_container_ref() {
            Some(container) => Some(container.container_id().to_string()),
            None => parent_id,
        };
        let media_id = item.media_id().map(str::to_string);
        let heos = self.heos.clone();
        self.add_bind(async move {
            let playable = Self::try_playable(&heos, playable_id).await?;
            let player_id = match playable.info() {
                PlayableInfo::Group(group) => group.leader().player_id,
                PlayableInfo::Player(player) => player.player_id,
            };
            heos.command(AddToQueue {
                player_id,
                source_id,
                container_id,
                media_id,
                add_to_queue_type,
            }).await
        });
    }
}