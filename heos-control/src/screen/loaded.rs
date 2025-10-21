use egui::{Context, Margin, Rgba, Stroke};
use heos::{HeosConnection, Stateful};
use std::sync::Arc;

use crate::assets;
use crate::screen::devices::Devices;
use crate::screen::media_bar::MediaBar;
use crate::util::normalized_gamma_multiply;

enum Screen {
    Devices(Devices),
}

pub struct Loaded {
    heos: Arc<HeosConnection<Stateful>>,
    media_bar: MediaBar,
    screen: Screen,
}

impl Loaded {
    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        let media_bar = MediaBar::new(heos.clone());
        let devices = Devices::new(heos.clone());
        let screen = Screen::Devices(devices);

        Self {
            heos,
            media_bar,
            screen,
        }
    }

    fn side_panel(&mut self, ctx: &Context) {
        egui::SidePanel::left("navigation")
            .resizable(false)
            .exact_width(48.0)
            .frame(egui::Frame {
                fill: normalized_gamma_multiply(ctx.style().visuals.panel_fill, 0.925),
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
                    let devices_button = egui::Button::new(assets::icons::devices::image())
                        .image_tint_follows_text_color(true)
                        .fill(Rgba::from_luminance_alpha(0.0, 0.0))
                        .stroke(Stroke::NONE);
                    if ui.add(devices_button).clicked() {
                        let devices = Devices::new(self.heos.clone());
                        self.screen = Screen::Devices(devices);
                    }
                });
            });
    }

    pub fn update(&mut self, ctx: &Context) {
        self.media_bar.update(ctx);
        self.side_panel(ctx);

        match &mut self.screen {
            Screen::Devices(devices) => {
                let selected = devices.update(ctx);

                if let Some(selected) = selected {
                    self.media_bar.set_active(selected);
                }
            },
        }
    }
}