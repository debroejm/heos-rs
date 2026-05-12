use std::sync::Arc;
use egui::{Layout, TextStyle, Ui, UiBuilder};
use egui_async::Bind;
use emath::Align;
use tracing::warn;
use heos::{HeosConnection, Stateful};
use heos::command::{CommandError, CommandErrorCode};
use heos::data::media::{MediaContainerBase, MediaItem, MediaItemBase};
use heos::data::source::{SearchCriteria, SourceId, SourceInfo};
use crate::screen::music::{BrowseSelection, ContainerSelection, PlaySelection, ScrollToRequest};
use crate::widgets::media::{MediaCarousal, MediaTileSelection};
use crate::widgets::SectionHeader;

struct Source {
    info: SourceInfo,
    categories: Option<Bind<Vec<Category>, CommandError>>
}

impl Source {
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

                    let criteria = source.search_criteria().await?;


                    let items = source.browse().await?.value;
                    let mut categories = vec![Category::misc(source_id)];
                    let mut misc = vec![];

                    for item in items {
                        match item.try_into_media_container() {
                            Ok(container) => categories.push(Category::new(source_id, container)),
                            Err(item) => misc.push(item),
                        }
                    }

                    let misc_preview = crate::screen::music::browse::top::CategoryPreview {
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
}

struct CategoryPreview {
    items: Vec<MediaItem>,
    total_count: usize,
}

struct Category {
    source_id: SourceId,
    search_str: String,
    criteria: SearchCriteria,
    preview: Option<Bind<CategoryPreview, CommandError>>,
}

impl Category {
    const PREVIEW_COUNT: usize = 20;

    fn new(source_id: SourceId, search_str: String, criteria: SearchCriteria) -> Self {
        Self {
            source_id,
            search_str,
            criteria,
            preview: None,
        }
    }

    fn check_loaded(&mut self, heos: &Arc<HeosConnection<Stateful>>, source_id: SourceId) -> bool {
        match &mut self.preview {
            Some(bind) => bind.is_finished(),
            None => {
                let heos = heos.clone();
                let search_str = self.search_str.clone();
                let criteria_id = self.criteria.criteria;
                let mut bind = Bind::new(true);
                bind.request(async move {
                    let source = heos.source(&source_id).await
                        .ok_or(CommandError::Failure {
                            code: CommandErrorCode::InvalidId,
                            text: format!("No source found for '{source_id}'"),
                        })?;

                    let response = source.search_range(
                        search_str,
                        criteria_id,
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
        selection: Option<MediaTileSelection<'_, T>>,
    ) -> Option<Vec<BrowseSelection>> {
        if let Some(selection) = selection {
            match selection {
                MediaTileSelection::PlayStation(station) => {
                    Some(vec![BrowseSelection::Play(PlaySelection::PlayStation {
                        source_id,
                        parent_id: None,
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
                    Some(vec![selection.into_browse_selection(source_id, None)])
                },
                MediaTileSelection::GoToContainer(container) => {
                    Some(vec![BrowseSelection::Container(ContainerSelection {
                        source_id,
                        container: container.into_media_container(),
                    })])
                },
                MediaTileSelection::SeeMore => {
                    todo!("this")
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

    fn show(&mut self, ui: &mut Ui, scroll_to: bool) -> Vec<BrowseSelection> {
        let criteria_name = self.criteria.name.clone();
        let add_header = |ui: &mut Ui| {
            let header = SectionHeader::text(criteria_name)
                .bg_color(ui.visuals().faint_bg_color.gamma_multiply(4.0))
                .text_style(TextStyle::Heading);
            let response = ui.add(header);
            if scroll_to {
                response.scroll_to_me(Some(Align::TOP));
            }
        };

        let ui_builder = UiBuilder::new()
            .id_salt(&self.criteria.name);
        if let Some(bind) = &mut self.preview {
            if let Some(result) = bind.read_as_mut() {
                return ui.scope_builder(ui_builder.layout(Layout::top_down_justified(Align::Min)), |ui| {
                    match result {
                        Ok(preview) => {
                            if !preview.items.is_empty() {
                                add_header(ui);

                                let csl_output = MediaCarousal::new(&preview.items)
                                    .count(preview.total_count)
                                    .show(ui);
                            }
                        }
                    }
                }).inner
            }
        }
    }
}

pub struct Search {
    heos: Arc<HeosConnection<Stateful>>,
    search_str: String,
    sources: Vec<Source>,
    scroll_to_request: Option<ScrollToRequest>
}