use egui::style::HandleShape;
use egui::{Align, Button, Color32, Context, Direction, Frame, Label, Layout, Slider, Stroke, Ui, UiBuilder};
use heos::data::common::{MuteState, Volume};
use heos::data::media::MediaItem;
use heos::data::player::{PlayState, RepeatMode, ShuffleMode};
use heos::data::queue::NowPlayingInfo;
use heos::state::playable::PlayableSnapshot;

use crate::actions::Actions;
use crate::assets;
use crate::widgets::media::MediaDisplay;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlButton {
    PlayPause,
    Next,
    Prev,
    Shuffle,
    Repeat,
}

struct ActiveMediaBar<'a> {
    snapshot: &'a PlayableSnapshot,
}

impl<'a> ActiveMediaBar<'a> {
    fn new(snapshot: &'a PlayableSnapshot) -> Self {
        Self {
            snapshot,
        }
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
        let (image, size, weak, enabled) = match button_type {
            ControlButton::PlayPause => {
                let image = match self.snapshot.play_state {
                    PlayState::Stop | PlayState::Pause => assets::icons::play::image(),
                    PlayState::Play => assets::icons::pause::image(),
                };
                (image, MediaBar::PLAY_BUTTON_SIZE, false, true)
            },
            ControlButton::Next => (assets::icons::next::image(), MediaBar::NEXT_PREV_BUTTON_SIZE, false, true),
            ControlButton::Prev => {
                let enabled = match &self.snapshot.now_playing.info {
                    NowPlayingInfo::Song { .. } => true,
                    NowPlayingInfo::Station { .. } => false,
                };
                (assets::icons::prev::image(), MediaBar::NEXT_PREV_BUTTON_SIZE, false, enabled)
            },
            ControlButton::Repeat => match &self.snapshot.now_playing.info {
                NowPlayingInfo::Song { .. } => match self.snapshot.repeat {
                    RepeatMode::Off => (assets::icons::repeat::image(), MediaBar::REPEAT_BUTTON_SIZE, true, true),
                    RepeatMode::One => (assets::icons::repeat_once::image(), MediaBar::REPEAT_BUTTON_SIZE, false, true),
                    RepeatMode::All => (assets::icons::repeat::image(), MediaBar::REPEAT_BUTTON_SIZE, false, true),
                },
                NowPlayingInfo::Station { .. } => (assets::icons::repeat::image(), MediaBar::REPEAT_BUTTON_SIZE, true, false),
            },
            ControlButton::Shuffle => {
                let weak = self.snapshot.shuffle == ShuffleMode::Off;
                let enabled = match &self.snapshot.now_playing.info {
                    NowPlayingInfo::Song { .. } => true,
                    NowPlayingInfo::Station { .. } => false,
                };
                (assets::icons::shuffle::image(), MediaBar::SHUFFLE_BUTTON_SIZE, weak, enabled)
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
            if ui.add_enabled(enabled, button).clicked() {
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
            Label::new(format!("{}:{:02}", elapsed.floor() as u64 / 60, elapsed.floor() as u64 % 60)).truncate(),
        );
        ui.add_enabled(duration > 0.0, slider);
        ui.add_sized(
            emath::vec2(text_width, ui.available_height()),
            Label::new(format!("{}:{:02}", duration.floor() as u64 / 60, duration.floor() as u64 % 60)).truncate(),
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

    fn volume(&mut self, ui: &mut Ui, actions: &mut Actions) {
        let margin = 30.0;
        let mut volume = self.snapshot.volume.into();

        ui.add_space(margin);

        let volume_icon = match self.snapshot.mute {
            MuteState::On => assets::icons::volume_mute::image(),
            MuteState::Off => if volume < 25 {
                assets::icons::volume_0::image()
            } else if volume < 50 {
                assets::icons::volume_1::image()
            } else if volume < 75 {
                assets::icons::volume_2::image()
            } else {
                assets::icons::volume_3::image()
            }
        };
        let volume_icon = volume_icon
            .max_size(emath::Vec2::splat(MediaBar::VOLUME_ICON_SIZE))
            .fit_to_exact_size(emath::Vec2::splat(MediaBar::VOLUME_ICON_SIZE));
        let volume_toggle = Button::new(volume_icon)
            .frame(false)
            .image_tint_follows_text_color(true);
        ui.scope(|ui| {
            if ui.add(volume_toggle).clicked() {
                actions.toggle_mute(self.snapshot.id);
            }
        });

        ui.add_space(10.0);

        ui.spacing_mut().slider_width = MediaBar::VOLUME_BAR_MAX_SIZE
            .min(ui.available_width() - margin);
        let slider = Slider::new(&mut volume, Volume::MIN..=Volume::MAX)
            .show_value(false)
            .trailing_fill(true)
            .handle_shape(HandleShape::Circle);
        if ui.add(slider).drag_stopped() {
            if let Ok(volume) = Volume::try_from(volume) {
                actions.set_volume(self.snapshot.id, volume);
            }
        }
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
                    top_left.x + corner_width * 2.0 + controls_width + spacing.x * 2.0,
                    bottom,
                ),
            ))
            .layout(Layout::right_to_left(Align::Center)));
        volume.set_width(corner_width);

        self.song_info(&mut info);
        self.controls(ctx, &mut controls, actions);
        self.volume(&mut volume, actions);

        ui.advance_cursor_after_rect(emath::Rect::from_min_size(
            top_left,
            available,
        ));
    }
}

pub struct MediaBar<'a> {
    snapshot: Option<&'a PlayableSnapshot>,
}

impl<'a> MediaBar<'a> {
    // Adjust these as needed
    const MIN_CORNER_WIDTH: f32 = 200.0;
    const MAX_CORNER_WIDTH: f32 = 300.0;
    const VOLUME_ICON_SIZE: f32 = 20.0;
    const VOLUME_BAR_MAX_SIZE: f32 = 120.0;
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

    pub fn new(snapshot: Option<&'a PlayableSnapshot>) -> Self {
        Self {
            snapshot,
        }
    }

    pub fn show(self, ctx: &Context, actions: &mut Actions) {
        let panel = egui::TopBottomPanel::bottom("media_bar")
            .resizable(false)
            .frame(Frame::side_top_panel(&ctx.style())
                .fill(ctx.style().visuals.panel_fill.gamma_multiply(1.2)))
            .show_separator_line(false)
            .exact_height(Self::MIN_SIZE.y);
        panel.show(ctx, |ui| {
            match self.snapshot {
                None => {
                    ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                        ui.heading("Select a Device");
                    });
                },
                Some(snapshot) => {
                    ActiveMediaBar::new(snapshot).show(ctx, ui, actions);
                }
            }
        });
    }
}