use egui::{Color32, ScrollArea, TextStyle, Ui};
use egui_async::Bind;
use heos::command::{CommandError, CommandErrorCode};
use heos::data::media::*;
use heos::data::source::SourceId;
use heos::{HeosConnection, Stateful};
use std::sync::Arc;
use tracing::warn;

use crate::screen::music::{BrowseSelection, ContainerSelection, MusicScreenLayer, MusicScreenLayerOutput, PlaySelection, ScrollToRequest};
use crate::widgets::media::{MediaGallery, MediaList, MediaTileSelection};
use crate::widgets::{ErrorSplash, SectionHeader};

#[derive(Default)]
struct BrowseContainerData {
    misc: Vec<MediaItem>,
    artists: Vec<Artist>,
    albums: Vec<Album>,
    songs: Vec<Song>,
    stations: Vec<Station>,
    genres: Vec<Genre>,
}

pub struct BrowseContainer {
    heos: Arc<HeosConnection<Stateful>>,
    source_id: SourceId,
    container: MediaContainer,
    data: Option<Bind<BrowseContainerData, CommandError>>,
}

impl BrowseContainer {
    pub fn new(
        heos: Arc<HeosConnection<Stateful>>,
        source_id: SourceId,
        container: MediaContainer,
    ) -> Self {
        Self {
            heos,
            source_id,
            container,
            data: None,
        }
    }

    fn tile_to_browse_select<T: MediaItemBase>(
        source_id: SourceId,
        parent_id: &String,
        selection: Option<MediaTileSelection<'_, T>>,
    ) -> Option<BrowseSelection> {
        if let Some(selection) = selection {
            match selection {
                MediaTileSelection::PlayStation(station) => {
                    Some(BrowseSelection::Play(PlaySelection::PlayStation {
                        source_id,
                        parent_id: Some(parent_id.clone()),
                        item: station.as_media_item_ref().into_media_item(),
                    }))
                },
                MediaTileSelection::PlayInputSource(input_source) => {
                    Some(BrowseSelection::Play(PlaySelection::PlayInputSource {
                        source_id,
                        item: input_source.as_media_item_ref().into_media_item(),
                    }))
                },
                MediaTileSelection::AddToQueue(selection) => {
                    Some(selection.into_browse_selection(source_id, Some(parent_id.clone())))
                },
                MediaTileSelection::GoToContainer(container) => {
                    Some(BrowseSelection::Container(ContainerSelection {
                        source_id,
                        container: container.into_media_container(),
                    }))
                },
                MediaTileSelection::SeeMore => {
                    warn!("SeeMore selected for BrowseContainer carousal that shouldn't display it");
                    None
                },
            }
        } else {
            None
        }
    }
}

impl MusicScreenLayer for BrowseContainer {
    fn check_loaded(&mut self) -> bool {
        if let Some(bind) = &mut self.data {
            match bind.get_state() {
                egui_async::State::Pending => return false,
                egui_async::State::Finished => return true,
                _ => {}
            }
        }

        let heos = self.heos.clone();
        let source_id = self.source_id;
        let container = self.container.clone();
        let mut data = Bind::new(true);
        data.request(async move {
            let source = heos.source(&source_id).await
                .ok_or(CommandError::Failure {
                    code: CommandErrorCode::InvalidId,
                    text: format!("No source found for '{source_id}'"),
                })?;

            let response = source.browse_container(
                container.container_id(),
            ).await?;

            let mut data = BrowseContainerData::default();
            for item in response.value {
                match item {
                    MediaItem::Artist(artist) => data.artists.push(artist),
                    MediaItem::Album(album) => data.albums.push(album),
                    MediaItem::Song(song) => data.songs.push(song),
                    MediaItem::Station(station) => data.stations.push(station),
                    MediaItem::Genre(genre) => data.genres.push(genre),
                    item => data.misc.push(item),
                }
            }

            Ok(data)
        });
        self.data = Some(data);

        false
    }

    fn show(&mut self, ui: &mut Ui) -> MusicScreenLayerOutput {
        let source_id = self.source_id;
        let parent_id = self.container.container_id().to_string();

        let header = SectionHeader::text(self.container.name())
            .bg_color(Color32::TRANSPARENT)
            .text_style(TextStyle::Name("LargeHeading".into()));
        ui.add(header);
        ui.separator();

        if let Some(bind) = &mut self.data {
            if let Some(result) = bind.read_as_mut() {
                let mut output_selection = None;
                match result {
                    Ok(data) => {
                        ScrollArea::vertical().id_salt("browse-container-scroll").show(ui, |ui| {
                            if !data.misc.is_empty() {
                                ui.push_id("misc", |ui| {
                                    let output = MediaGallery::new(&data.misc).show(ui);
                                    if let Some(selection) = Self::tile_to_browse_select(source_id, &parent_id, output.selection) {
                                        output_selection = Some(selection);
                                    }
                                });
                            }

                            if !data.genres.is_empty() {
                                ui.push_id("genres", |ui| {
                                    ui.add(
                                        SectionHeader::text("Genres")
                                            .bg_color(ui.visuals().faint_bg_color.gamma_multiply(4.0))
                                    );
                                    let output = MediaGallery::new(&data.genres).show(ui);
                                    if let Some(selection) = Self::tile_to_browse_select(source_id, &parent_id, output.selection) {
                                        output_selection = Some(selection);
                                    }
                                });
                            }

                            if !data.stations.is_empty() {
                                ui.push_id("stations", |ui| {
                                    ui.add(
                                        SectionHeader::text("Stations")
                                            .bg_color(ui.visuals().faint_bg_color.gamma_multiply(4.0))
                                    );
                                    let output = MediaGallery::new(&data.stations).show(ui);
                                    if let Some(selection) = Self::tile_to_browse_select(source_id, &parent_id, output.selection) {
                                        output_selection = Some(selection);
                                    }
                                });
                            }

                            if !data.artists.is_empty() {
                                ui.push_id("artists", |ui| {
                                    ui.add(
                                        SectionHeader::text("Artists")
                                            .bg_color(ui.visuals().faint_bg_color.gamma_multiply(4.0))
                                    );
                                    let output = MediaGallery::new(&data.artists).show(ui);
                                    if let Some(selection) = Self::tile_to_browse_select(source_id, &parent_id, output.selection) {
                                        output_selection = Some(selection);
                                    }
                                });
                            }

                            if !data.albums.is_empty() {
                                ui.push_id("albums", |ui| {
                                    ui.add(
                                        SectionHeader::text("Albums")
                                            .bg_color(ui.visuals().faint_bg_color.gamma_multiply(4.0))
                                    );
                                    let output = MediaGallery::new(&data.albums).show(ui);
                                    if let Some(selection) = Self::tile_to_browse_select(source_id, &parent_id, output.selection) {
                                        output_selection = Some(selection);
                                    }
                                });
                            }

                            if !data.songs.is_empty() {
                                ui.push_id("songs", |ui| {
                                    let output = MediaList::new(&data.songs).show(ui);
                                    if let Some(selection) = output.selection {
                                        output_selection = Some(selection.into_browse_selection(source_id, Some(parent_id)));
                                    }
                                });
                            }
                        });
                    },
                    Err(err) => {
                        ui.add(ErrorSplash::new(err).centered());
                    },
                }
                return MusicScreenLayerOutput {
                    selections: output_selection.into_iter().collect(),
                }
            }
        }

        ui.spinner();
        MusicScreenLayerOutput::default()
    }

    fn scroll_to(&mut self, _request: ScrollToRequest) {
        warn!("Received ScrollToRequest for BrowseContainer; discarding");
    }

    fn title(&mut self) -> &str {
        self.container.name()
    }
}