use egui::style::HandleShape;
use egui::{Align, Button, Color32, Context, Direction, Frame, Label, Layout, RichText, Slider, Stroke, Ui, UiBuilder};
use egui_async::Bind;
use heos::data::event::Event;
use heos::data::media::MediaItem;
use heos::data::player::{PlayState, RepeatMode, ShuffleMode};
use heos::data::queue::NowPlayingInfo;
use heos::state::playable::{PlayableId, PlayableSnapshot};
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::warn;

use crate::actions::Actions;
use crate::assets;
use crate::updater::Updater;
use crate::widgets::MediaDisplay;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlButton {
    PlayPause,
    Next,
    Prev,
    Shuffle,
    Repeat,
}

struct ActiveData {
    snapshot: PlayableSnapshot,
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
            snapshot,
        })
    }

    fn song_info(&self, ui: &mut Ui) {
        match &self.snapshot.now_playing.info {
            NowPlayingInfo::Song { info, .. } | NowPlayingInfo::Station { info, .. } => {
                let item = MediaItem::from(info.clone());
                ui.add(MediaDisplay::new(&item));
            }
        }
    }

    fn control_button(&mut self, ui: &mut Ui, actions: &mut Actions, button_type: ControlButton) {
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
                actions.media_control(self.snapshot.id, button_type);
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

    fn controls(&mut self, ctx: &Context, ui: &mut Ui, actions: &mut Actions) {
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
        self.control_button(&mut buttons, actions, ControlButton::Repeat);
        self.control_button(&mut buttons, actions, ControlButton::Prev);
        self.control_button(&mut buttons, actions, ControlButton::PlayPause);
        self.control_button(&mut buttons, actions, ControlButton::Next);
        self.control_button(&mut buttons, actions, ControlButton::Shuffle);

        self.song_progress(ctx, &mut song_progress);

        ui.advance_cursor_after_rect(emath::Rect::from_min_size(
            top_left,
            available,
        ));
    }

    fn show(&mut self, ctx: &Context, ui: &mut Ui, actions: &mut Actions) {
        let spacing = ui.spacing().item_spacing;
        let available = ui.available_size();
        let top_left = ui.cursor().min;
        let bottom = ui.max_rect().right_bottom().y;

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
        self.controls(ctx, &mut controls, actions);

        ui.advance_cursor_after_rect(emath::Rect::from_min_size(
            top_left,
            available,
        ));
    }
}

enum State {
    Inactive,
    Active {
        playable_id: PlayableId,
        data: Arc<Mutex<Bind<ActiveData, ()>>>,
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

    pub fn set_active(&mut self, updater: &Updater, playable_id: PlayableId) {
        let data = {
            let heos = self.heos.clone();
            let mut data_bind = Bind::new(false);
            data_bind.request(async move { ActiveData::new(heos, playable_id).await });
            Arc::new(Mutex::new(data_bind))
        };

        let heos = self.heos.clone();
        updater.register(
            &data,
            move |event| async move {
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
                    Event::GroupVolumeChanged(_) => true,
                    _ => false,
                }
            },
            move || ActiveData::new(heos.clone(), playable_id),
        );

        self.state = State::Active {
            playable_id,
            data,
        }
    }

    fn show(&mut self, ctx: &Context, ui: &mut Ui, actions: &mut Actions) {
        match &mut self.state {
            State::Inactive => {
                ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                    ui.heading("Select a Device");
                });
            },
            State::Active { data, .. } => {
                if let Some(result) = data.lock().read_mut() {
                    if let Ok(data) = result {
                        data.show(ctx, ui, actions);
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

    pub fn update(&mut self, ctx: &Context, actions: &mut Actions) {
        egui::TopBottomPanel::bottom("media_bar")
            .resizable(false)
            .frame(Frame::side_top_panel(&ctx.style())
                .fill(ctx.style().visuals.panel_fill.gamma_multiply(1.2)))
            .show_separator_line(false)
            .exact_height(Self::MIN_SIZE.y)
            .show(ctx, |ui| self.show(ctx, ui, actions));
    }
    
    pub fn playable_id(&self) -> Option<PlayableId> {
        match &self.state {
            State::Inactive => None,
            State::Active { playable_id, .. } => Some(*playable_id),
        }
    }
}