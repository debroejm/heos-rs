use egui_async::Bind;
use heos::command::group::SetGroup;
use heos::command::{CommandError, CommandErrorCode};
use heos::data::group::GroupRole;
use heos::data::player::{PlayState, PlayerId, RepeatMode, ShuffleMode};
use heos::state::playable::{Playable, PlayableId};
use heos::{HeosConnection, Stateful};
use std::iter;
use std::sync::Arc;
use egui_async::bind::MaybeSend;
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
}