use egui::{Button, Color32, Context, Frame, Id, Layout, Response, ScrollArea, Sense, Sides, Stroke, Ui, Widget};
use egui_async::Bind;
use egui_dnd::{Dnd, DragDropItem, Handle};
use emath::Align;
use heos::data::event::Event;
use heos::data::media::MediaItem;
use heos::data::queue::{NowPlayingInfo, QueueId, QueuedTrackInfo};
use heos::state::playable::{PlayableId, PlayableInfo, PlayableSnapshot};
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::convert::Infallible;
use std::sync::Arc;

use crate::actions::Actions;
use crate::assets;
use crate::updater::Updater;
use crate::widgets::frame::TileFrame;
use crate::widgets::media::MediaDisplay;

struct QueuedTrackDnd<'a>(&'a QueuedTrackInfo);

impl<'a> DragDropItem for QueuedTrackDnd<'a> {
    fn id(&self) -> Id {
        Id::new(self.0.queue_id)
    }
}

pub struct QueuedTrack<'a> {
    playable_id: PlayableId,
    track: &'a QueuedTrackInfo,
    handle: Handle<'a>,
    actions: &'a mut Actions,
    striped: bool,
    selected: bool,
}

impl<'a> QueuedTrack<'a> {
    const HEIGHT: f32 = 60.0;

    pub fn new(
        playable_id: PlayableId,
        track: &'a QueuedTrackInfo,
        handle: Handle<'a>,
        actions: &'a mut Actions,
    ) -> Self {
        Self {
            playable_id,
            track,
            handle,
            actions,
            striped: false,
            selected: false,
        }
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl<'a> Widget for QueuedTrack<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let frame = TileFrame::default()
            .layout(Layout::left_to_right(Align::Min))
            .sense(Sense::hover())
            .bg_color(ui.style().visuals.faint_bg_color.gamma_multiply(match self.striped {
                true => 4.0,
                false => 2.0,
            }))
            .selected(self.selected);

        frame.show(ui, |ui| {
            ui.set_height(Self::HEIGHT);
            ui.set_width(ui.available_width());

            Sides::new().shrink_left().show(
                ui,
                |ui| {
                    ui.set_height(Self::HEIGHT);
                    // LEFT
                    let item = MediaItem::from(self.track.clone());
                    ui.add(MediaDisplay::new(&item));
                },
                |ui| {
                    // RIGHT
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_height(Self::HEIGHT);
                        ui.add_space(20.0);

                        self.handle.ui(ui, |ui| {
                            let image = assets::icons::grab::image()
                                .max_size(emath::Vec2::splat(Self::HEIGHT / 1.5))
                                .fit_to_exact_size(emath::Vec2::splat(Self::HEIGHT / 1.5))
                                .tint(ui.visuals().faint_bg_color.gamma_multiply(6.0));
                            ui.add(image);
                        });

                        ui.add_space(20.0);

                        ui.scope(|ui| {
                            let image = assets::icons::trash::image()
                                .max_size(emath::Vec2::splat(Self::HEIGHT / 3.0))
                                .fit_to_exact_size(emath::Vec2::splat(Self::HEIGHT / 3.0));
                            let delete_button = Button::new(image)
                                .frame(false)
                                .image_tint_follows_text_color(true);
                            ui.style_mut().visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::DARK_RED);
                            ui.style_mut().visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::RED);
                            if ui.add(delete_button).clicked() {
                                self.actions.remove_from_queue(self.playable_id, self.track.queue_id);
                            }
                        });
                    });
                },
            );
        }).response
    }
}

impl<'a> DragDropItem for QueuedTrack<'a> {
    fn id(&self) -> Id {
        Id::new(self.track.queue_id)
    }
}

pub struct QueueScreen {
    queue: Arc<Mutex<Bind<Vec<QueuedTrackInfo>, Infallible>>>,
}

impl QueueScreen {
    async fn query_queue(
        heos: Arc<HeosConnection<Stateful>>,
        playable_id: PlayableId,
    ) -> Result<Vec<QueuedTrackInfo>, Infallible> {
        if let Some(player) = heos.playable(playable_id).await {
            Ok(player.queue().data().await.clone())
        } else {
            Ok(vec![])
        }
    }

    pub fn new(heos: Arc<HeosConnection<Stateful>>, updater: &Updater, playable: &PlayableSnapshot) -> Self {
        let queue = {
            let heos = heos.clone();
            let playable_id = playable.id;
            let mut queue_bind = Bind::new(false);
            queue_bind.request(async move { Self::query_queue(heos, playable_id).await });
            Arc::new(Mutex::new(queue_bind))
        };

        {
            let heos_update_fn = heos.clone();
            let playable_id = playable.id;
            let player_id = match &playable.info {
                PlayableInfo::Player(player) => player.player_id,
                PlayableInfo::Group(group) => group.leader().player_id,
            };
            updater.register(
                &queue,
                move |event| {
                    async move {
                        match event {
                            Event::PlayerQueueChanged(queue_changed) => {
                                queue_changed.player_id == player_id
                            },
                            _ => false,
                        }
                    }
                },
                move || Self::query_queue(heos_update_fn.clone(), playable_id),
            )
        }

        Self {
            queue,
        }
    }

    fn show(&mut self, ui: &mut Ui, actions: &mut Actions, playable: &PlayableSnapshot) {
        let now_playing_queue_id = match &playable.now_playing.info {
            NowPlayingInfo::Song { info, .. } => Some(info.queue_id),
            NowPlayingInfo::Station { .. } => None,
        };
        if let Some(queue) = self.queue.lock().read() {
            let queue = queue.as_ref().unwrap();
            ScrollArea::vertical().show(ui, |ui| {
                let iter = queue.iter().map(QueuedTrackDnd);
                let response = Dnd::new(ui, "queue_dnd").show(iter, |ui, track, handle, state| {
                    let track = QueuedTrack::new(playable.id, track.0, handle, actions)
                        .striped(state.index % 2 != 0)
                        .selected(match now_playing_queue_id {
                            Some(id) => track.0.queue_id == id,
                            None => false,
                        });
                    ui.add(track);
                });
                if let Some(update) = response.final_update() {
                    println!("Dragged: {} => {}", update.from, update.to);
                    let from = queue[update.from].queue_id;
                    let to = if update.to < queue.len() {
                        if update.to > update.from {
                            // HEOS CLI is weird, and moves the track to the _far_ side of the
                            // destination relative to the src, so the ID needs to be adjusted to
                            // account for that
                            QueueId::from(queue[update.to].queue_id.0 - 1)
                        } else {
                            queue[update.to].queue_id
                        }
                    } else {
                        // QueueIds are numbered `1..` for items in the queue ('0' represents the
                        // now playing slot). For moving a track to the very end, we have to create
                        // a new ID that represents beyond the end of the current queue
                        QueueId::from(queue.len() as u64)
                    };
                    actions.move_queue(playable.id, from, to);
                }
            });
        } else {
            ui.spinner();
        }
    }

    pub fn update(&mut self, ctx: &Context, actions: &mut Actions, playable: &PlayableSnapshot) {
        egui::CentralPanel::default()
            .frame(Frame::central_panel(&ctx.style())
                .fill(ctx.style().visuals.extreme_bg_color))
            .show(ctx, |ui| self.show(ui, actions, playable));
    }
}