use eframe::epaint::text::TextWrapMode;
use egui::{Context, FontSelection, Frame, Grid, Label, Layout, Response, RichText, ScrollArea, Sense, TextStyle, Ui, UiBuilder, Widget, WidgetText};
use egui_async::Bind;
use emath::Align;
use heos::data::event::Event;
use heos::data::group::GroupPlayer;
use heos::data::media::MediaItem;
use heos::data::player::PlayerId;
use heos::data::queue::NowPlayingInfo;
use heos::state::playable::{PlayableId, PlayableInfo, PlayableSnapshot};
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::convert::Infallible;
use std::sync::Arc;

use crate::actions::Actions;
use crate::updater::Updater;
use crate::widgets::MediaDisplay;

pub struct SubDevice<'a> {
    player: &'a GroupPlayer,
}

impl<'a> SubDevice<'a> {
    pub fn new(player: &'a GroupPlayer) -> Self {
        Self {
            player,
        }
    }
}

impl<'a> Widget for SubDevice<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let margin = 8.0;

        let mut frame = Frame::NONE
            .inner_margin(margin)
            .corner_radius(8.0)
            .begin(ui);

        let available_width = frame.content_ui.available_width();
        let galley = WidgetText::from(RichText::new(&self.player.name).heading()).into_galley(
            &mut frame.content_ui,
            None,
            available_width,
            FontSelection::Default,
        );

        let inner_response = frame.content_ui.scope_builder(
            UiBuilder::new().sense(Sense::drag() | Sense::hover()),
            |ui| {
                ui.add(Label::new(galley).selectable(false))
            }
        ).response;
        let response = frame.allocate_space(ui);
        if response.hovered() {
            frame.frame.fill = ui.style().visuals.selection.bg_fill;
        } else {
            frame.frame.fill = ui.style().visuals.panel_fill + ui.style().visuals.faint_bg_color.gamma_multiply(4.0);
        }
        frame.paint(ui);
        response | inner_response
    }
}

pub struct Device<'a> {
    idx: usize,
    playable: &'a PlayableSnapshot,
}

impl<'a> Device<'a> {
    const HEIGHT: f32 = 80.0;

    pub fn new(idx: usize, playable: &'a PlayableSnapshot) -> Self {
        Self {
            idx,
            playable,
        }
    }

    fn ui_left(&self, ui: &mut Ui) -> bool {
        if let PlayableInfo::Group(group) = &self.playable.info {
            let response = group.players.iter()
                .map(|player| {
                    let response = ui.add(SubDevice::new(player));
                    response.dnd_set_drag_payload(player.player_id);
                    response
                })
                .reduce(|a, b| a | b);
            match response {
                Some(response) => response.hovered(),
                None => false,
            }
        } else {
            false
        }
    }

    fn ui_right(&self, ui: &mut Ui) {
        let track_info = match &self.playable.now_playing.info {
            NowPlayingInfo::Station { info, .. } |
            NowPlayingInfo::Song { info, .. } => info,
        };
        let item = MediaItem::from(track_info.clone());
        ui.add(MediaDisplay::new(&item));
    }
}

impl<'a> Widget for Device<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let margin = 8.0;
        let mut frame = Frame::NONE
            .inner_margin(margin)
            .corner_radius(6.0)
            .begin(ui);
        let inner = frame.content_ui.scope_builder(
            UiBuilder::new()
                .layout(
                    Layout::top_down(Align::Min)
                        .with_cross_justify(true)
                )
                .sense(Sense::click() | Sense::hover() | Sense::drag()),
            |ui| {
                ui.set_height(Self::HEIGHT);

                let available = ui.available_size();
                let top_left = ui.cursor().min;

                let title_height = { // Title
                    let text = match &self.playable.info {
                        PlayableInfo::Player(player) => player.name.as_str(),
                        PlayableInfo::Group(group) => group.name.as_str(),
                    };

                    let galley = WidgetText::from(
                        RichText::new(text)
                            .text_style(TextStyle::Heading)
                            .strong()
                    ).into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        available.x,
                        FontSelection::Default,
                    );
                    let title_height = galley.rect.height() + margin;

                    let mut title = ui.new_child(
                        UiBuilder::new()
                            .max_rect(emath::Rect::from_min_max(
                                top_left,
                                emath::pos2(top_left.x + available.x, top_left.y + title_height),
                            ))
                            .layout(Layout::left_to_right(Align::Center))
                    );
                    title.set_width(available.x);
                    title.set_height(title_height);
                    title.add(Label::new(galley).selectable(false));
                    title_height
                };

                let mut sub_hovered = false;
                if title_height < available.y {
                    let top_left = emath::pos2(top_left.x, top_left.y + title_height);
                    let seg_width = available.x / 2.0;
                    let bottom_right = ui.max_rect().right_bottom();

                    let mut left = ui.new_child(
                        UiBuilder::new()
                            .max_rect(emath::Rect::from_min_max(
                                top_left,
                                emath::pos2(top_left.x + seg_width, bottom_right.y),
                            ))
                            .layout(Layout::left_to_right(Align::TOP).with_main_wrap(true))
                    );
                    left.set_width(seg_width);
                    sub_hovered = self.ui_left(&mut left);

                    let mut right = ui.new_child(
                        UiBuilder::new()
                            .max_rect(emath::Rect::from_min_max(
                                emath::pos2(top_left.x + seg_width, top_left.y),
                                bottom_right,
                            ))
                            .layout(Layout::right_to_left(Align::Center))
                    );
                    right.set_width(seg_width);
                    self.ui_right(&mut right);
                }

                ui.advance_cursor_after_rect(emath::Rect::from_min_size(
                    top_left,
                    available,
                ));

                sub_hovered
            }
        );
        let response = frame.allocate_space(ui);
        if response.hovered() && !inner.inner {
            frame.frame.fill = ui.style().visuals.selection.bg_fill;
        } else if self.idx % 2 != 0 {
            frame.frame.fill = ui.style().visuals.faint_bg_color.gamma_multiply(4.0);
        } else {
            frame.frame.fill = ui.style().visuals.faint_bg_color.gamma_multiply(2.0);
        }
        frame.paint(ui);
        response | inner.response
    }
}

pub struct Devices {
    devices: Arc<Mutex<Bind<Vec<PlayableSnapshot>, Infallible>>>,
}

impl Devices {
    async fn query_devices(heos: Arc<HeosConnection<Stateful>>) -> Result<Vec<PlayableSnapshot>, Infallible> {
        let mut snapshots = vec![];
        for playable in heos.playables().await {
            let snapshot = playable.snapshot().await;
            snapshots.push(snapshot);
        }
        snapshots.sort_by_cached_key(|snapshot| {
            match &snapshot.info {
                PlayableInfo::Player(player) => player.name.clone(),
                PlayableInfo::Group(group) => group.name.clone(),
            }
        });
        Ok(snapshots)
    }

    pub fn new(heos: Arc<HeosConnection<Stateful>>, updater: &Updater) -> Self {
        let devices = {
            let heos = heos.clone();
            let mut devices_bind = Bind::new(false);
            devices_bind.request(async move { Self::query_devices(heos).await });
            Arc::new(Mutex::new(devices_bind))
        };

        updater.register(
            &devices,
            move |event| async move {
                match event {
                    Event::PlayersChanged |
                    Event::GroupsChanged => true,
                    _ => false,
                }
            },
            move || Self::query_devices(heos.clone()),
        );

        Self {
            devices,
        }
    }

    fn show(&mut self, ui: &mut Ui, actions: &mut Actions) -> Option<PlayableId> {
        let mut selected = None;

        ui.spacing_mut().scroll.foreground_color = true;

        let mut group_adds = vec![];
        let mut group_removes = vec![];

        if let Some(devices) = self.devices.lock().read() {
            let devices = devices.as_ref().unwrap();
            ScrollArea::vertical().show(ui, |ui| {
                let dnd = Grid::new("devices_grid")
                    .num_columns(1)
                    .show(ui, |ui| {
                        let mut dnd = false;
                        for (idx, device) in devices.iter().enumerate() {
                            let response = ui.add(Device::new(idx, device));
                            {
                                let player_id = match &device.info {
                                    PlayableInfo::Group(_) => None,
                                    PlayableInfo::Player(player) => Some(player.player_id),
                                };
                                if let Some(player_id) = player_id {
                                    response.dnd_set_drag_payload(player_id);
                                }
                            }
                            if response.clicked() {
                                selected = Some(device.id);
                            }
                            if let Some(payload) = response.dnd_release_payload::<PlayerId>() {
                                group_adds.push((device.id, *payload));
                                dnd = true;
                            }
                            ui.end_row();
                        }
                        dnd
                    }).inner;

                if !dnd {
                    let response = ui.allocate_response(ui.available_size(), Sense::empty());
                    if let Some(payload) = response.dnd_release_payload::<PlayerId>() {
                        group_removes.push(*payload);
                    }
                }
            });

        } else {
            ui.spinner();
        }

        for (playable_id, new_player_id) in group_adds {
            actions.add_to_group(playable_id, new_player_id);
        }

        for player_id in group_removes {
            actions.remove_from_any_group(player_id);
        }

        selected
    }

    pub fn update(&mut self, ctx: &Context, actions: &mut Actions) -> Option<PlayableId> {
        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style())
                .fill(ctx.style().visuals.extreme_bg_color))
            .show(ctx, |ui| self.show(ui, actions)).inner
    }
}