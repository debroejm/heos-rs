use egui::{Layout, ScrollArea, TextStyle, Ui, UiBuilder};
use egui_async::Bind;
use emath::Align;
use heos::command::{CommandError, CommandErrorCode};
use heos::data::media::{MediaContainer, MediaContainerBase, MediaItem, MediaItemBase};
use heos::data::source::{SourceAvailable, SourceId, SourceInfo};
use heos::{HeosConnection, Stateful};
use std::sync::Arc;
use tracing::warn;

use crate::screen::music::{BrowseSelection, ContainerSelection, MusicScreenLayer, MusicScreenLayerOutput, PlaySelection, ScrollToRequest};
use crate::widgets::media::{MediaCarousal, MediaTileSelection};
use crate::widgets::{ErrorSplash, SectionHeader};

#[derive(Default)]
struct SourceOutput {
    selections: Vec<BrowseSelection>,
}

struct Source {
    info: SourceInfo,
    categories: Option<Bind<Vec<Category>, CommandError>>,
}

impl Source {
    const SPACING: f32 = 50.0;

    fn new(info: SourceInfo) -> Self {
        Self {
            info,
            categories: None,
        }
    }

    fn check_loaded(&mut self, heos: &Arc<HeosConnection<Stateful>>) -> bool {
        match &mut self.categories {
            Some(bind) => {
                if let Some(result) = bind.read_as_mut() {
                    match result {
                        Ok(categories) => {
                            for category in categories {
                                if !category.check_loaded(heos, self.info.source_id) {
                                    // returning early here will ensure only one category is
                                    // actively loaded data at a time
                                    return false
                                }
                            }
                            true
                        },
                        Err(_) => true,
                    }
                } else {
                    false
                }
            },
            None => {
                let heos = heos.clone();
                let source_id = self.info.source_id;
                let mut bind = Bind::new(true);
                bind.request(async move {
                    let source = heos.source(&source_id).await
                        .ok_or(CommandError::Failure {
                            code: CommandErrorCode::InvalidId,
                            text: format!("No source found for '{source_id}'"),
                        })?;

                    let items = source.browse().await?.value;
                    let mut categories = vec![Category::misc(source_id)];
                    let mut misc = vec![];

                    for item in items {
                        match item.try_into_media_container() {
                            Ok(container) => categories.push(Category::new(source_id, container)),
                            Err(item) => misc.push(item),
                        }
                    }

                    let misc_preview = CategoryPreview {
                        total_count: misc.len(),
                        items: misc,
                    };
                    let mut misc_bind = Bind::new(true);
                    misc_bind.fill(Ok(misc_preview));
                    categories[0].preview = Some(misc_bind);

                    Ok(categories)
                });
                self.categories = Some(bind);
                false
            }
        }
    }

    fn check_category_scroll_request(
        scroll_to_request: &Option<ScrollToRequest>,
        container_id: &str,
    ) -> bool {
        if let Some(request) = scroll_to_request {
            if let Some(request_container_id) = &request.container_id {
                return request_container_id == container_id
            }
        }

        false
    }

    fn show(
        &mut self,
        ui: &mut Ui,
        scroll_to_request: Option<ScrollToRequest>,
    ) -> SourceOutput {
        let source_id = self.info.source_id;
        let source_name = self.info.name.clone();

        let add_header = |ui: &mut Ui| {
            let header_text_style = TextStyle::Name("LargeHeading".into());
            let header = SectionHeader::from_source(&self.info)
                .bg_color(ui.visuals().faint_bg_color.gamma_multiply(8.0))
                // Leave extra spacing around text, and room for larger images
                .height(ui.text_style_height(&header_text_style) * 2.0)
                .text_style(header_text_style);
            let response = ui.add(header);
            if scroll_to_request.is_some() {
                response.scroll_to_me(Some(Align::TOP));
            }
        };

        let ui_builder = UiBuilder::new()
            .id_salt(&self.info.name);
        if let Some(bind) = &mut self.categories {
            if let Some(result) = bind.read_as_mut() {
                return ui.scope_builder(ui_builder.layout(Layout::top_down_justified(Align::Min)), |ui| {
                    let mut selections = vec![];
                    match result {
                        Ok(categories) => {
                            if !categories.iter_mut().all(Category::check_empty) {
                                add_header(ui);
                                for category in categories {
                                    let mut scroll_to = false;
                                    if let Some(container) = &category.container {
                                        scroll_to = Self::check_category_scroll_request(
                                            &scroll_to_request,
                                            container.container_id(),
                                        );
                                    }
                                    let category_output = category.show(
                                        ui,
                                        scroll_to,
                                    );
                                    if !category_output.selections.is_empty() {
                                        let scroll_to = match category_output.selections.first().unwrap() {
                                            BrowseSelection::Container(_) => {
                                                Some(BrowseSelection::ScrollTo(ScrollToRequest {
                                                    source_id,
                                                    container_id: None,
                                                    breadcrumb: source_name.clone(),
                                                }))
                                            },
                                            _ => None,
                                        };
                                        selections = scroll_to.into_iter()
                                            .chain(category_output.selections.into_iter())
                                            .collect::<Vec<_>>();
                                    }
                                }
                                ui.add_space(Source::SPACING);
                            } // else do nothing as the source is empty
                        },
                        Err(err) => {
                            add_header(ui);
                            ui.add(ErrorSplash::new(err));
                            ui.add_space(Source::SPACING);
                        },
                    }
                    SourceOutput {
                        selections,
                    }
                }).inner
            }
        }

        ui.scope_builder(ui_builder.layout(Layout::top_down_justified(Align::Center)), |ui| {
            add_header(ui);
            ui.spinner();
            ui.add_space(Source::SPACING);
        });
        SourceOutput::default()
    }
}

#[derive(Default)]
struct CategoryOutput {
    selections: Vec<BrowseSelection>,
}

struct CategoryPreview {
    items: Vec<MediaItem>,
    total_count: usize,
}

struct Category {
    source_id: SourceId,
    container: Option<MediaContainer>,
    preview: Option<Bind<CategoryPreview, CommandError>>,
}

impl Category {
    const SPACING: f32 = 20.0;
    const PREVIEW_COUNT: usize = 20;

    fn misc(source_id: SourceId) -> Self {
        Self {
            source_id,
            container: None,
            preview: None,
        }
    }

    fn new(source_id: SourceId, container: MediaContainer) -> Self {
        Self {
            source_id,
            container: Some(container),
            preview: None,
        }
    }

    fn check_loaded(&mut self, heos: &Arc<HeosConnection<Stateful>>, source_id: SourceId) -> bool {
        match &mut self.preview {
            Some(bind) => bind.is_finished(),
            None => {
                let heos = heos.clone();
                let container = self.container.as_ref()
                    .expect("container should not be None if bind is not finished");
                let container_id = container.container_id().to_string();
                let mut bind = Bind::new(true);
                bind.request(async move {
                    let source = heos.source(&source_id).await
                        .ok_or(CommandError::Failure {
                            code: CommandErrorCode::InvalidId,
                            text: format!("No source found for '{source_id}'"),
                        })?;

                    let response = source.browse_container_range(
                        container_id,
                        0..=(Self::PREVIEW_COUNT - 1),
                    ).await?.value;

                    Ok(CategoryPreview {
                        items: response.items,
                        total_count: response.count,
                    })
                });
                self.preview = Some(bind);
                false
            }
        }
    }

    fn check_empty(&mut self) -> bool {
        if self.source_id == SourceId::HeosAuxInputs {
            // TODO: Aux Inputs are currently broken, as the HEOS apis don't provide possible input
            //  names, so we have no way of specifying which aux input to select
            return true
        }

        if let Some(bind) = &mut self.preview {
            if let Some(result) = bind.read_as_mut() {
                match result {
                    Ok(preview) => preview.items.is_empty(),
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    fn tile_to_browse_select<T: MediaItemBase>(
        source_id: SourceId,
        parent: &Option<MediaContainer>,
        selection: Option<MediaTileSelection<'_, T>>,
    ) -> Option<Vec<BrowseSelection>> {
        if let Some(selection) = selection {
            match selection {
                MediaTileSelection::PlayStation(station) => {
                    Some(vec![BrowseSelection::Play(PlaySelection::PlayStation {
                        source_id,
                        parent_id: parent.as_ref().map(|p| p.container_id().to_string()),
                        item: station.as_media_item_ref().into_media_item(),
                    })])
                },
                MediaTileSelection::PlayInputSource(input_source) => {
                    Some(vec![BrowseSelection::Play(PlaySelection::PlayInputSource {
                        source_id,
                        item: input_source.as_media_item_ref().into_media_item(),
                    })])
                },
                MediaTileSelection::AddToQueue(selection) => {
                    Some(vec![selection.into_browse_selection(
                        source_id,
                        parent.as_ref().map(|p| p.container_id().to_string()),
                    )])
                },
                MediaTileSelection::GoToContainer(container) => {
                    let select = BrowseSelection::Container(ContainerSelection {
                        source_id,
                        container: container.into_media_container(),
                    });
                    Some(match parent.as_ref() {
                        Some(parent) => vec![
                            BrowseSelection::Container(ContainerSelection {
                                source_id,
                                container: parent.clone(),
                            }),
                            select,
                        ],
                        None => vec![select],
                    })
                },
                MediaTileSelection::SeeMore => {
                    match parent.as_ref() {
                        Some(parent) => Some(vec![BrowseSelection::Container(ContainerSelection {
                            source_id,
                            container: parent.clone(),
                        })]),
                        None => {
                            warn!("SeeMore selected for Browse carousal that doesn't represent a container");
                            None
                        }
                    }
                },
            }
        } else {
            None
        }
    }

    fn show(&mut self, ui: &mut Ui, scroll_to: bool) -> CategoryOutput {
        let parent = self.container.clone();

        let add_header = |ui: &mut Ui| {
            let header = self.container.as_ref().map(|container| {
                SectionHeader::text(container.name())
                    .bg_color(ui.visuals().faint_bg_color.gamma_multiply(4.0))
                    .text_style(TextStyle::Heading)
            });
            if let Some(header) = header {
                let response = ui.add(header);
                if scroll_to {
                    response.scroll_to_me(Some(Align::TOP));
                }
            } else if scroll_to {
                ui.scroll_to_cursor(Some(Align::TOP));
            }
        };

        let ui_builder = UiBuilder::new()
            .id_salt(
                self.container.as_ref()
                    .map(|container| container.name())
                    .unwrap_or("misc")
            );
        if let Some(bind) = &mut self.preview {
            if let Some(result) = bind.read_as_mut() {
                return ui.scope_builder(ui_builder.layout(Layout::top_down_justified(Align::Min)), |ui| {
                    let mut selections = None;
                    match result {
                        Ok(preview) => {
                            if !preview.items.is_empty() {
                                add_header(ui);

                                let csl_output = MediaCarousal::new(&preview.items)
                                    .count(preview.total_count)
                                    .show(ui);
                                if let Some(cat_select) = Self::tile_to_browse_select(
                                    self.source_id,
                                    &parent,
                                    csl_output.selection,
                                ) {
                                    selections = Some(cat_select);
                                }

                                ui.add_space(Category::SPACING);
                            } // else do nothing as the category is empty
                        },
                        Err(err) => {
                            add_header(ui);
                            ui.add(ErrorSplash::new(err));
                            ui.add_space(Category::SPACING);
                        },
                    }

                    CategoryOutput {
                        selections: selections.unwrap_or(vec![]),
                    }
                }).inner
            }
        }

        ui.scope_builder(ui_builder.layout(Layout::top_down_justified(Align::Center)), |ui| {
            add_header(ui);
            ui.spinner();
            ui.add_space(Category::SPACING);
        });
        CategoryOutput::default()
    }
}

pub struct BrowseTop {
    heos: Arc<HeosConnection<Stateful>>,
    sources: Vec<Source>,
    scroll_to_request: Option<ScrollToRequest>
}

impl BrowseTop {
    pub async fn init(heos: Arc<HeosConnection<Stateful>>) -> Self {
        let mut sources = heos.sources().await
            .filter(|source| source.info().available == SourceAvailable::True)
            .map(|source| Source::new(source.info().clone()))
            .collect::<Vec<_>>();

        sources.sort_by_cached_key(|source| i64::from(source.info.source_id));

        Self {
            heos,
            sources,
            scroll_to_request: None,
        }
    }

    fn check_and_take_scroll_request(
        scroll_to_request: &mut Option<ScrollToRequest>,
        source_id: SourceId,
    ) -> Option<ScrollToRequest> {
        if let Some(request) = scroll_to_request {
            if request.source_id == source_id {
                return scroll_to_request.take()
            }
        }

        None
    }
}

impl MusicScreenLayer for BrowseTop {
    fn check_loaded(&mut self) -> bool {
        for source in &mut self.sources {
            if !source.check_loaded(&self.heos) {
                // returning early here will ensure only one source is actively loaded data at a time
                return false
            }
        }
        true
    }

    fn show(&mut self, ui: &mut Ui) -> MusicScreenLayerOutput {
        let mut selections = vec![];
        ScrollArea::vertical().id_salt("browse-top-scroll").show(ui, |ui| {
            let mut scroll_to_request = self.scroll_to_request.take();
            for source in &mut self.sources {
                let source_output = source.show(
                    ui,
                    Self::check_and_take_scroll_request(&mut scroll_to_request, source.info.source_id),
                );
                if !source_output.selections.is_empty() {
                    selections = source_output.selections;
                }
            }
        });
        MusicScreenLayerOutput {
            selections,
        }
    }

    fn scroll_to(&mut self, request: ScrollToRequest) {
        self.scroll_to_request = Some(request);
    }

    fn title(&mut self) -> &str {
        "Browse"
    }
}