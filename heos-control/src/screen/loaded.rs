use egui::{Button, Color32, Context, Image, Margin, Response, Rgba, Stroke, Ui, Widget};
use heos::{HeosConnection, Stateful};
use std::sync::Arc;
use emath::Vec2;
use strum::{EnumDiscriminants, IntoDiscriminant};

use crate::actions::Actions;
use crate::assets;
use crate::screen::devices::Devices;
use crate::screen::media_bar::MediaBar;
use crate::screen::queue::QueueScreen;
use crate::updater::Updater;

#[derive(EnumDiscriminants)]
#[strum_discriminants(name(ScreenType))]
enum Screen {
    Devices(Devices),
    Queue(QueueScreen),
}

impl ScreenType {
    fn image(&self) -> Image<'static> {
        match self {
            Self::Devices => assets::icons::devices::image(),
            Self::Queue => assets::icons::queue::image(),
        }
    }
}

struct SidePanelButton {
    screen_type: ScreenType,
    enabled: bool,
    selected: bool,
}

impl SidePanelButton {
    fn new(screen_type: ScreenType) -> Self {
        Self {
            screen_type,
            enabled: true,
            selected: false,
        }
    }

    fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn selected(mut self, screen: &Screen) -> Self {
        self.selected = screen.discriminant() == self.screen_type;
        self
    }
}

impl Widget for SidePanelButton {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(4.0, 4.0);
            let visuals = ui.visuals_mut();
            visuals.button_frame = true;
            visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
            visuals.widgets.hovered.bg_stroke = Stroke::NONE;
            visuals.widgets.active.bg_stroke = Stroke::NONE;
            visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);

            let button = Button::new(self.screen_type.image())
                .image_tint_follows_text_color(true)
                .corner_radius(0.0)
                .selected(self.selected);
            ui.add_enabled(self.enabled, button)
        }).inner
    }
}

pub struct Loaded {
    heos: Arc<HeosConnection<Stateful>>,
    media_bar: MediaBar,
    screen: Screen,
}

impl Loaded {
    pub fn new(heos: Arc<HeosConnection<Stateful>>, updater: &Updater) -> Self {
        let media_bar = MediaBar::new(heos.clone());
        let devices = Devices::new(heos.clone(), updater);
        let screen = Screen::Devices(devices);

        Self {
            heos,
            media_bar,
            screen,
        }
    }

    fn side_panel(&mut self, ctx: &Context, updater: &Updater) {
        egui::SidePanel::left("navigation")
            .resizable(false)
            .exact_width(48.0)
            .frame(egui::Frame {
                fill: ctx.style().visuals.panel_fill.gamma_multiply(0.8),
                inner_margin: Margin::ZERO,
                outer_margin: Margin::ZERO,
                shadow: egui::Shadow {
                    offset: [10, 0],
                    blur: 10,
                    spread: 0,
                    color: Rgba::from_black_alpha(1.0).into(),
                },
                ..egui::Frame::side_top_panel(&ctx.style())
            })
            .show_separator_line(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.add_space(4.0);

                    let devices_button = SidePanelButton::new(ScreenType::Devices)
                        .selected(&self.screen);
                    if ui.add(devices_button).clicked() {
                        let devices = Devices::new(self.heos.clone(), updater);
                        self.screen = Screen::Devices(devices);
                    }

                    let playable_id = self.media_bar.playable_id();
                    let queue_button = SidePanelButton::new(ScreenType::Queue)
                        .selected(&self.screen)
                        .enabled(playable_id.is_some());
                    if ui.add(queue_button).clicked() {
                        if let Some(playable_id) = playable_id {
                            let queue = QueueScreen::new(self.heos.clone(), updater, playable_id);
                            self.screen = Screen::Queue(queue);
                        }
                    }
                });
            });
    }

    pub fn update(&mut self, ctx: &Context, actions: &mut Actions, updater: &Updater) {
        self.media_bar.update(ctx, actions);
        self.side_panel(ctx, updater);

        match &mut self.screen {
            Screen::Devices(devices) => {
                let selected = devices.update(ctx, actions);

                if let Some(selected) = selected {
                    self.media_bar.set_active(updater, selected);
                }
            },
            Screen::Queue(queue) => {
                queue.update(ctx, actions);
            }
        }
    }
}