use egui::{Context, CursorIcon, Grid, Label, ScrollArea, Sense, TextStyle, Ui};
use egui_async::Bind;
use heos::data::event::Event;
use heos::state::playable::{PlayableId, PlayableInfo, PlayableSnapshot};
use heos::{HeosConnection, Stateful};
use parking_lot::Mutex;
use std::convert::Infallible;
use std::sync::Arc;

use crate::util::Updater;

pub struct Devices {
    devices: Arc<Mutex<Bind<Vec<PlayableSnapshot>, Infallible>>>,
    updater: Updater,
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

    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        let devices = {
            let heos = heos.clone();
            let mut devices_bind = Bind::new(false);
            devices_bind.request(async move { Self::query_devices(heos).await });
            Arc::new(Mutex::new(devices_bind))
        };

        let updater = {
            let heos = heos.clone();
            let devices = Arc::downgrade(&devices);
            Updater::new(heos.clone(), move |event| {
                let heos = heos.clone();
                let devices = devices.clone();
                async move {
                    match event {
                        Event::PlayersChanged |
                        Event::GroupsChanged => {
                            if let Some(devices) = devices.upgrade() {
                                let heos = heos.clone();
                                devices.lock().request(async move {
                                    Self::query_devices(heos).await
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

        Self {
            devices,
            updater,
        }
    }

    fn show(&mut self, ui: &mut Ui) -> Option<PlayableId> {
        let mut selected = None;

        if let Some(devices) = self.devices.lock().read() {
            let devices = devices.as_ref().unwrap();
            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("devices_grid")
                    .striped(true)
                    .num_columns(1)
                    .show(ui, |ui| {
                        for device in devices {
                            ui.centered_and_justified(|ui| {
                                {
                                    let style = ui.style_mut();
                                    style.override_text_style = Some(TextStyle::Heading);
                                    style.visuals.interact_cursor = Some(CursorIcon::Alias);
                                }
                                let text = match &device.info {
                                    PlayableInfo::Player(player) => player.name.as_str(),
                                    PlayableInfo::Group(group) => group.name.as_str(),
                                };
                                let label = Label::new(text)
                                    .truncate()
                                    .selectable(false)
                                    .sense(Sense::click());
                                if ui.add(label).clicked() {
                                    selected = Some(device.id);
                                }
                            });

                            ui.end_row();
                        }
                    });
            });
        } else {
            ui.spinner();
        }

        selected
    }

    pub fn update(&mut self, ctx: &Context) -> Option<PlayableId> {
        egui::CentralPanel::default().show(ctx, |ui| self.show(ui)).inner
    }
}

impl Drop for Devices {
    fn drop(&mut self) {
        self.updater.stop();
    }
}