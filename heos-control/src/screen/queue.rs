use egui::{Context, Frame, Id, Layout, Response, ScrollArea, Sense, Ui, Widget};
use egui_async::Bind;
use egui_dnd::{Dnd, DragDropItem};
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
use crate::updater::Updater;
use crate::widgets::frame::TileFrame;
use crate::widgets::media::MediaDisplay;

pub struct QueuedTrack<'a> {
    track: &'a QueuedTrackInfo,
    striped: bool,
    selected: bool,
}

impl<'a> QueuedTrack<'a> {
    const HEIGHT: f32 = 60.0;

    pub fn new(track: &'a QueuedTrackInfo) -> Self {
        Self {
            track,
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

            let item = MediaItem::from(self.track.clone());
            ui.add(MediaDisplay::new(&item));
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
                let tracks = queue.iter().enumerate()
                    .map(|(idx, track)| {
                        QueuedTrack::new(track)
                            .striped(idx % 2 != 0)
                            .selected(match now_playing_queue_id {
                                Some(id) => track.queue_id == id,
                                None => false,
                            })
                    });
                let response = Dnd::new(ui, "queue_dnd").show(tracks, |ui, track, handle, _| {
                    handle.ui(ui, |ui| {
                        ui.add(track);
                    });
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