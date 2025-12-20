use egui::{Button, Color32, Context, Image, Margin, Response, Rgba, Stroke, Ui, Widget};
use egui_async::Bind;
use emath::Vec2;
use heos::data::event::Event;
use heos::state::playable::{PlayableId, PlayableSnapshot};
use heos::{HeosConnection, Stateful};
use parking_lot::{Mutex, MutexGuard};
use std::convert::Infallible;
use std::sync::Arc;
use strum::{EnumDiscriminants, IntoDiscriminant};
use tracing::warn;

use crate::actions::Actions;
use crate::assets;
use crate::screen::devices::Devices;
use crate::screen::media_bar::MediaBar;
use crate::screen::music::{MusicScreen, MusicScreenStack};
use crate::screen::queue::QueueScreen;
use crate::updater::Updater;

pub struct ActivePlayable {
    heos: Arc<HeosConnection<Stateful>>,
    target_id: Arc<Mutex<Option<PlayableId>>>,
    bind: Arc<Mutex<Bind<Option<PlayableSnapshot>, Infallible>>>,
}

impl ActivePlayable {
    async fn refresh_playable(
        heos: Arc<HeosConnection<Stateful>>,
        target_id: Arc<Mutex<Option<PlayableId>>>,
    ) -> Result<Option<PlayableSnapshot>, Infallible> {
        let target_id = target_id.lock().clone();
        if let Some(target_id) = target_id {
            if let Some(playable) = heos.playable(target_id).await {
                return Ok(Some(playable.snapshot().await))
            }
        }
        Ok(None)
    }

    fn new(heos: Arc<HeosConnection<Stateful>>, updater: &Updater) -> Self {
        let mut bind = Bind::new(true);
        bind.fill(Ok(None));
        let active_playable = Self {
            heos: heos.clone(),
            target_id: Arc::new(Mutex::new(None)),
            bind: Arc::new(Mutex::new(bind)),
        };

        {
            let target_id = active_playable.target_id.clone();
            updater.register(
                &active_playable.bind,
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
                move || Self::refresh_playable(heos.clone(), target_id.clone()),
            );
        }

        active_playable
    }

    pub fn set_target_id(&self, playable_id: PlayableId) {
        *self.target_id.lock() = Some(playable_id);
        self.bind.lock().request(Self::refresh_playable(self.heos.clone(), self.target_id.clone()));
    }

    pub fn lock(&self) -> ActivePlayableGuard<'_> {
        ActivePlayableGuard {
            inner: self.bind.lock(),
        }
    }
}

pub struct ActivePlayableGuard<'a> {
    inner: MutexGuard<'a, Bind<Option<PlayableSnapshot>, Infallible>>,
}

impl<'a> ActivePlayableGuard<'a> {
    pub fn read(&mut self) -> Option<&PlayableSnapshot> {
        match self.inner.read() {
            Some(result) => {
                result.as_ref().unwrap().as_ref()
            },
            None => None,
        }
    }
}

#[derive(EnumDiscriminants)]
#[strum_discriminants(name(ScreenType))]
enum Screen {
    Devices(Devices),
    Music(MusicScreen),
    Queue(QueueScreen),
    Pending,
}

impl ScreenType {
    fn image(&self) -> Image<'static> {
        match self {
            Self::Devices => assets::icons::devices::image(),
            Self::Music => assets::icons::music::image(),
            Self::Queue => assets::icons::queue::image(),
            // This screen isn't manually navigable, so the icon image doesn't matter
            Self::Pending => assets::icons::devices::image(),
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
    active_playable: ActivePlayable,
    screen: Screen,
    music_screen_stack: Option<MusicScreenStack>,
}

impl Loaded {
    pub fn new(heos: Arc<HeosConnection<Stateful>>, updater: &Updater) -> Self {
        let active_playable = ActivePlayable::new(heos.clone(), updater);
        let devices = Devices::new(heos.clone(), updater);
        let screen = Screen::Devices(devices);

        Self {
            heos,
            active_playable,
            screen,
            music_screen_stack: None,
        }
    }

    fn set_screen(&mut self, screen_type: ScreenType, updater: &Updater) {
        let mut screen = Screen::Pending;
        std::mem::swap(&mut screen, &mut self.screen);

        // Clean up old screen if needed
        match screen {
            Screen::Music(music) => {
                self.music_screen_stack = Some(music.into_stack());
            },
            _ => {}
        }

        self.screen = match screen_type {
            ScreenType::Devices => Screen::Devices(Devices::new(self.heos.clone(), updater)),
            ScreenType::Music => {
                if let Some(playable) = self.active_playable.lock().read() {
                    Screen::Music(MusicScreen::new(
                        self.heos.clone(),
                        self.music_screen_stack.take(),
                        playable.id,
                    ))
                } else {
                    warn!("Tried to set screen to 'Music' without having a playable ID selected");
                    Screen::Pending
                }
            },
            ScreenType::Queue => {
                if let Some(playable) = self.active_playable.lock().read() {
                    Screen::Queue(QueueScreen::new(self.heos.clone(), updater, playable.id))
                } else {
                    warn!("Tried to set screen to 'Queue' without having a playable ID selected");
                    Screen::Pending
                }
            },
            ScreenType::Pending => Screen::Pending,
        };
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
                        self.set_screen(ScreenType::Devices, updater);
                    }

                    let enabled = self.active_playable.lock().read().is_some();

                    let music_button = SidePanelButton::new(ScreenType::Music)
                        .selected(&self.screen)
                        .enabled(enabled);
                    if ui.add(music_button).clicked() {
                        self.set_screen(ScreenType::Music, updater);
                    }

                    let queue_button = SidePanelButton::new(ScreenType::Queue)
                        .selected(&self.screen)
                        .enabled(enabled);
                    if ui.add(queue_button).clicked() {
                        self.set_screen(ScreenType::Queue, updater);
                    }

                    if !enabled {
                        // Kick the screen back to devices if the selected playable is no longer valid
                        match self.screen.discriminant() {
                            ScreenType::Music | ScreenType::Queue =>
                                self.set_screen(ScreenType::Devices, updater),
                            _ => {},
                        }
                    }
                });
            });
    }

    pub fn update(&mut self, ctx: &Context, actions: &mut Actions, updater: &Updater) {
        MediaBar::new(self.active_playable.lock().read()).show(ctx, actions);
        self.side_panel(ctx, updater);

        match &mut self.screen {
            Screen::Devices(devices) => {
                let selected = devices.update(ctx, actions);

                if let Some(selected) = selected {
                    self.active_playable.set_target_id(selected);
                }
            },
            Screen::Music(music) => {
                music.update(ctx, actions);
            },
            Screen::Queue(queue) => {
                queue.update(ctx, actions);
            },
            Screen::Pending => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.spinner();
                });
            },
        }
    }
}