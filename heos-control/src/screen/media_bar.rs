use egui::style::HandleShape;
use egui::{Align, Button, Color32, Context, Direction, Frame, Image, Label, Layout, RichText, Slider, Stroke, Ui, UiBuilder};
use egui_async::Bind;
use heos::command::CommandError;
use heos::data::event::Event;
use heos::data::player::{PlayState, RepeatMode, ShuffleMode};
use heos::data::song::NowPlayingInfo;
use heos::state::playable::{PlayableId, PlayableSnapshot};
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::warn;

use crate::assets;
use crate::util::{normalized_gamma_multiply, Updater};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlButton {
    PlayPause,
    Next,
    Prev,
    Shuffle,
    Repeat,
}

struct ActiveData {
    heos: Arc<HeosConnection<Stateful>>,
    snapshot: PlayableSnapshot,
    tasks: Vec<Bind<(), CommandError>>,
}

impl ActiveData {
    async fn new(
        heos: Arc<HeosConnection<Stateful>>,
        playable_id: PlayableId,
    ) -> Result<Self, ()> {
        let snapshot = {
            let playable = match heos.playable(playable_id).await {
                Some(playable) => playable,
                None => {
                    warn!(?playable_id, "No playable found for ID");
                    return Err(())
                }
            };
            playable.snapshot().await
        };

        Ok(Self {
            heos,
            snapshot,
            tasks: vec![],
        })
    }

    fn song_info(&self, ui: &mut Ui) {
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            match &self.snapshot.now_playing.info {
                NowPlayingInfo::Song { info, .. } | NowPlayingInfo::Station { info, .. } => {
                    if let Some(image) = info.image_url.as_ref().map(|url| Image::from_uri(url.as_str())) {
                        ui.add(image);
                    }

                    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                        ui.add_space(20.0);
                        ui.add(Label::new(RichText::new(&info.song).strong()).truncate());
                        ui.add(Label::new(&info.album).truncate());
                        ui.add(Label::new(&info.artist).truncate());
                    });
                }
            }
        });
    }

    fn control_button(&mut self, ui: &mut Ui, button_type: ControlButton) {
        let (image, size, weak) = match button_type {
            ControlButton::PlayPause => {
                let image = match self.snapshot.play_state {
                    PlayState::Stop | PlayState::Pause => assets::icons::play::image(),
                    PlayState::Play => assets::icons::pause::image(),
                };
                (image, MediaBar::PLAY_BUTTON_SIZE, false)
            },
            ControlButton::Next => (assets::icons::next::image(), MediaBar::NEXT_PREV_BUTTON_SIZE, false),
            ControlButton::Prev => (assets::icons::prev::image(), MediaBar::NEXT_PREV_BUTTON_SIZE, false),
            ControlButton::Repeat => match self.snapshot.repeat {
                RepeatMode::Off => (assets::icons::repeat::image(), MediaBar::REPEAT_BUTTON_SIZE, true),
                RepeatMode::One => (assets::icons::repeat_once::image(), MediaBar::REPEAT_BUTTON_SIZE, false),
                RepeatMode::All => (assets::icons::repeat::image(), MediaBar::REPEAT_BUTTON_SIZE, false),
            },
            ControlButton::Shuffle => {
                let weak = self.snapshot.shuffle == ShuffleMode::Off;
                (assets::icons::shuffle::image(), MediaBar::SHUFFLE_BUTTON_SIZE, weak)
            },
        };
        let image = image
            .max_size(emath::Vec2::splat(size))
            .fit_to_exact_size(emath::Vec2::splat(size));
        let button = Button::new(image)
            .fill(Color32::from_black_alpha(0))
            .stroke(Stroke::NONE)
            .frame(false)
            .image_tint_follows_text_color(true);
        ui.scope(|ui| {
            if weak {
                let style = ui.style_mut();
                let weak_color = style.visuals.weak_text_color();
                style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, weak_color);
            }
            if ui.add(button).clicked() {
                let heos = self.heos.clone();
                let snapshot = self.snapshot.clone();
                let mut task = Bind::new(true);
                task.request(async move {
                    let playable = match heos.playable(snapshot.id).await {
                        Some(playable) => playable,
                        None => {
                            warn!(
                            id = %snapshot.id,
                            error = "no player found for ID",
                            "Failed to play/pause",
                        );
                            return Err(CommandError::InvalidId)
                        }
                    };

                    let result = match button_type {
                        ControlButton::PlayPause => {
                            playable.set_play_state(match snapshot.play_state {
                                PlayState::Stop | PlayState::Pause => PlayState::Play,
                                PlayState::Play => PlayState::Pause,
                            }).await
                        },
                        ControlButton::Next => playable.play_next().await,
                        ControlButton::Prev => playable.play_previous().await,
                        ControlButton::Repeat => playable.set_play_mode(
                            Some(match snapshot.repeat {
                                RepeatMode::Off => RepeatMode::One,
                                RepeatMode::One => RepeatMode::All,
                                RepeatMode::All => RepeatMode::Off,
                            }),
                            None,
                        ).await,
                        ControlButton::Shuffle => playable.set_play_mode(
                            None,
                            Some(match snapshot.shuffle {
                                ShuffleMode::Off => ShuffleMode::On,
                                ShuffleMode::On => ShuffleMode::Off,
                            }),
                        ).await,
                    };

                    match result {
                        Ok(_) => Ok(()),
                        Err(error) => {
                            warn!(id = %snapshot.id, ?button_type, ?error, "Operation failed");
                            Err(error)
                        }
                    }
                });
                self.tasks.push(task);
            }
        });
    }

    fn song_progress(&self, ctx: &Context, ui: &mut Ui) {
        let text_width = 30.0;
        let slider_width = ui.available_width() - (text_width * 2.0);

        ui.style_mut().spacing.slider_width = slider_width;

        let elapsed = self.snapshot.now_playing.progress.interpolated_elapsed().as_secs_f32() as f64;
        let duration = self.snapshot.now_playing.progress.duration.as_secs_f32() as f64;
        let slider_max = if duration == 0.0 {
            // Fix so the slider has a valid range
            1.0
        } else {
            duration
        };
        let slider = Slider::from_get_set(
            0.0..=slider_max,
            |_| elapsed,
        );
        let slider = slider
            .show_value(false)
            .drag_value_speed(0.0)
            .trailing_fill(true)
            .handle_shape(HandleShape::Rect { aspect_ratio: 0.0 });
        ui.add_sized(
            emath::vec2(text_width, ui.available_height()),
            Label::new(format!("{}:{}", elapsed.floor() as u64 / 60, elapsed.floor() as u64 % 60)).truncate(),
        );
        ui.add_enabled(duration > 0.0, slider);
        ui.add_sized(
            emath::vec2(text_width, ui.available_height()),
            Label::new(format!("{}:{}", duration.floor() as u64 / 60, duration.floor() as u64 % 60)).truncate(),
        );

        if self.snapshot.play_state == PlayState::Play {
            ctx.request_repaint_after_secs(0.1);
        }
    }

    fn controls(&mut self, ctx: &Context, ui: &mut Ui) {
        let available = ui.available_size();
        let top_left = ui.cursor().min;
        let right_bottom = ui.max_rect().right_bottom();
        let buttons_offset = (available.x - MediaBar::CONTROLS_BUTTONS_SIZE.x) / 2.0;

        let mut buttons = ui.new_child(UiBuilder::new()
            .max_rect(emath::Rect::from_min_max(
                top_left + emath::vec2(buttons_offset, 0.0),
                top_left + emath::vec2(
                    buttons_offset + MediaBar::CONTROLS_BUTTONS_SIZE.x,
                    MediaBar::CONTROLS_BUTTONS_SIZE.y,
                ),
            ))
            .layout(Layout::left_to_right(Align::Center)));
        buttons.set_min_size(MediaBar::CONTROLS_BUTTONS_SIZE);
        buttons.set_max_size(MediaBar::CONTROLS_BUTTONS_SIZE);

        let mut song_progress = ui.new_child(UiBuilder::new()
            .max_rect(emath::Rect::from_min_max(
                top_left + emath::vec2(0.0, MediaBar::CONTROLS_BUTTONS_SIZE.y),
                right_bottom,
            ))
            .layout(Layout::left_to_right(Align::Center)));
        song_progress.set_width(available.x);
        song_progress.set_height(MediaBar::SONG_PROGRESS_HEIGHT);

        buttons.style_mut().spacing.item_spacing = MediaBar::BUTTON_SPACING;
        self.control_button(&mut buttons, ControlButton::Repeat);
        self.control_button(&mut buttons, ControlButton::Prev);
        self.control_button(&mut buttons, ControlButton::PlayPause);
        self.control_button(&mut buttons, ControlButton::Next);
        self.control_button(&mut buttons, ControlButton::Shuffle);

        self.song_progress(ctx, &mut song_progress);

        ui.advance_cursor_after_rect(emath::Rect::from_min_size(
            top_left,
            available,
        ));
    }

    fn show(&mut self, ctx: &Context, ui: &mut Ui) {
        let spacing = ui.spacing().item_spacing;
        let available = ui.available_size();
        let top_left = ui.cursor().min;
        let bottom = ui.max_rect().right_bottom().y;

        self.tasks.retain_mut(|task| {
            !task.is_finished()
        });

        let (corner_width, controls_width) = if available.x < (MediaBar::CONTROLS_BUTTONS_SIZE.x + MediaBar::MAX_CORNER_WIDTH * 2.0) {
            ((available.x - MediaBar::CONTROLS_BUTTONS_SIZE.x) / 2.0, MediaBar::CONTROLS_BUTTONS_SIZE.x)
        } else {
            (MediaBar::MAX_CORNER_WIDTH, available.x - (MediaBar::MAX_CORNER_WIDTH * 2.0))
        };

        let mut info = ui.new_child(UiBuilder::new()
            .max_rect(emath::Rect::from_min_max(
                top_left,
                emath::pos2(top_left.x + corner_width, bottom),
            ))
            .layout(Layout::left_to_right(Align::Center)));
        info.set_width(corner_width);

        let mut controls = ui.new_child(UiBuilder::new()
            .max_rect(emath::Rect::from_min_max(
                top_left + emath::vec2(corner_width + spacing.x, 0.0),
                emath::pos2(
                    top_left.x + corner_width + spacing.x + controls_width,
                    bottom,
                ),
            ))
            .layout(Layout::top_down_justified(Align::LEFT)));
        controls.set_width(controls_width);

        let mut volume = ui.new_child(UiBuilder::new()
            .max_rect(emath::Rect::from_min_max(
                top_left + emath::vec2(
                    corner_width + controls_width + spacing.x * 2.0,
                    0.0,
                ),
                emath::pos2(
                    top_left.x + corner_width + controls_width + spacing.x * 2.0,
                    bottom,
                ),
            ))
            .layout(Layout::right_to_left(Align::Center)));
        volume.set_width(corner_width);

        self.song_info(&mut info);
        self.controls(ctx, &mut controls);

        ui.advance_cursor_after_rect(emath::Rect::from_min_size(
            top_left,
            available,
        ));
    }
}

enum State {
    Inactive,
    Active {
        data: Arc<Mutex<Bind<ActiveData, ()>>>,
        updater: Updater,
    },
}

pub struct MediaBar {
    heos: Arc<HeosConnection<Stateful>>,
    state: State,
}

impl MediaBar {
    // Adjust these as needed
    const MIN_CORNER_WIDTH: f32 = 200.0;
    const MAX_CORNER_WIDTH: f32 = 300.0;
    const PLAY_BUTTON_SIZE: f32 = 40.0;
    const NEXT_PREV_BUTTON_SIZE: f32 = 20.0;
    const SHUFFLE_BUTTON_SIZE: f32 = Self::NEXT_PREV_BUTTON_SIZE;
    const REPEAT_BUTTON_SIZE: f32 = Self::SHUFFLE_BUTTON_SIZE;
    const BUTTON_SPACING: emath::Vec2 = emath::vec2(20.0, 12.0);
    const CONTROLS_BUTTONS_SIZE: emath::Vec2 = emath::vec2(
        Self::PLAY_BUTTON_SIZE
            + (Self::NEXT_PREV_BUTTON_SIZE * 2.0)
            + Self::SHUFFLE_BUTTON_SIZE
            + Self::REPEAT_BUTTON_SIZE
            + Self::BUTTON_SPACING.x * 4.0,
        Self::PLAY_BUTTON_SIZE
            .max(Self::NEXT_PREV_BUTTON_SIZE)
            .max(Self::SHUFFLE_BUTTON_SIZE)
            .max(Self::REPEAT_BUTTON_SIZE)
            + Self::BUTTON_SPACING.y * 2.0,
    );
    const SONG_PROGRESS_HEIGHT: f32 = 32.0;

    pub const MIN_SIZE: emath::Vec2 = emath::vec2(
        Self::CONTROLS_BUTTONS_SIZE.x + Self::MIN_CORNER_WIDTH * 2.0,
        Self::CONTROLS_BUTTONS_SIZE.y + Self::SONG_PROGRESS_HEIGHT,
    );

    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        Self {
            heos,
            state: State::Inactive,
        }
    }

    pub fn set_active(&mut self, playable_id: PlayableId) {
        if let State::Active { updater, .. } = &mut self.state {
            updater.stop();
        }

        let data = {
            let heos = self.heos.clone();
            let mut data_bind = Bind::new(false);
            data_bind.request(async move { ActiveData::new(heos, playable_id).await });
            Arc::new(Mutex::new(data_bind))
        };

        let updater = {
            let heos = self.heos.clone();
            let data = Arc::downgrade(&data);
            Updater::new(heos.clone(), move |event| {
                let heos = heos.clone();
                let data = data.clone();
                async move {
                    match event {
                        Event::PlayersChanged |
                        Event::GroupsChanged |
                        Event::PlayerStateChanged(_) |
                        Event::PlayerNowPlayingChanged(_) |
                        Event::PlayerNowPlayingProgress(_) |
                        Event::PlayerPlaybackError(_) |
                        Event::PlayerQueueChanged(_) |
                        Event::PlayerVolumeChanged(_) |
                        Event::PlayerRepeatModeChanged(_) |
                        Event::PlayerShuffleModeChanged(_) |
                        Event::GroupVolumeChanged(_) => {
                            if let Some(data) = data.upgrade() {
                                let heos = heos.clone();
                                data.lock().request(async move {
                                    ActiveData::new(heos, playable_id).await
                                });
                                false
                            } else {
                                // Nothing to update anymore
                                true
                            }
                        },
                        _ => false,
                    }
                }
            })
        };

        self.state = State::Active {
            data,
            updater,
        }
    }

    fn show(&mut self, ctx: &Context, ui: &mut Ui) {
        match &mut self.state {
            State::Inactive => {
                ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                    ui.heading("Select a Device");
                });
            },
            State::Active { data, .. } => {
                if let Some(result) = data.lock().read_mut() {
                    if let Ok(data) = result {
                        data.show(ctx, ui);
                    } else {
                        ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                            ui.heading(RichText::new("ERROR").color(Color32::RED));
                        });
                    }
                } else {
                    ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                        ui.spinner();
                    });
                }
            }
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        egui::TopBottomPanel::bottom("media_bar")
            .resizable(false)
            .frame(Frame {
                fill: normalized_gamma_multiply(ctx.style().visuals.panel_fill, 0.85),
                ..Frame::side_top_panel(&ctx.style())
            })
            .show_separator_line(false)
            .exact_height(Self::MIN_SIZE.y)
            .show(ctx, |ui| self.show(ctx, ui));
    }
}

impl Drop for MediaBar {
    fn drop(&mut self) {
        if let State::Active { updater, .. } = &mut self.state {
            updater.stop();
        }
    }
}