mod browse;
mod search;

use egui::{Context, FontSelection, Label, Layout, Response, RichText, Sense, TextEdit, TextStyle, Ui, Widget};
use egui_async::Bind;
use emath::Align;
use heos::data::media::{MediaContainer, MediaItem};
use heos::data::player::AddToQueueType;
use heos::data::source::SourceId;
use heos::state::playable::PlayableId;
use heos::{HeosConnection, Stateful};
use std::convert::Infallible;
use std::fmt::Display;
use std::sync::Arc;
use tracing::warn;

use crate::actions::Actions;
use crate::screen::music::browse::container::BrowseContainer;
use crate::screen::music::browse::top::BrowseTop;
use crate::widgets::frame::{TileFrame, TileFrameShape};
use crate::widgets::ErrorSplash;

#[derive(Debug, Clone)]
pub enum PlaySelection {
    PlayStation {
        source_id: SourceId,
        parent_id: Option<String>,
        item: MediaItem,
    },
    PlayInputSource {
        source_id: SourceId,
        item: MediaItem,
    },
    AddToQueue {
        source_id: SourceId,
        parent_id: Option<String>,
        item: MediaItem,
        operation: AddToQueueType,
    },
}

#[derive(Debug, Clone)]
pub struct ContainerSelection {
    pub source_id: SourceId,
    pub container: MediaContainer,
}

#[derive(Debug, Clone)]
pub struct ScrollToRequest {
    pub source_id: SourceId,
    pub container_id: Option<String>,
    pub breadcrumb: String,
}

#[derive(Debug, Clone)]
pub enum BrowseSelection {
    Play(PlaySelection),
    Container(ContainerSelection),
    ScrollTo(ScrollToRequest),
}

#[derive(Default)]
pub struct MusicScreenLayerOutput {
    pub selections: Vec<BrowseSelection>,
}

pub trait MusicScreenLayer {
    fn check_loaded(&mut self) -> bool;
    fn show(&mut self, ui: &mut Ui) -> MusicScreenLayerOutput;
    fn scroll_to(&mut self, request: ScrollToRequest);
    fn title(&mut self) -> &str;
}

impl<T, E> MusicScreenLayer for Bind<T, E>
where
    T: MusicScreenLayer + 'static,
    E: Display + 'static,
{
    fn check_loaded(&mut self) -> bool {
        if let Some(result) = self.read_as_mut() {
            match result {
                Ok(inner) => inner.check_loaded(),
                Err(_) => true,
            }
        } else {
            false
        }
    }

    fn show(&mut self, ui: &mut Ui) -> MusicScreenLayerOutput {
        if let Some(result) = self.read_as_mut() {
            match result {
                Ok(inner) => inner.show(ui),
                Err(err) => {
                    ui.add(ErrorSplash::new(err).centered());
                    MusicScreenLayerOutput::default()
                },
            }
        } else {
            ui.spinner();
            MusicScreenLayerOutput::default()
        }
    }

    fn scroll_to(&mut self, request: ScrollToRequest) {
        if let Some(result) = self.read_as_mut() {
            if let Ok(inner) = result {
                inner.scroll_to(request);
            }
        }
    }

    fn title(&mut self) -> &str {
        if let Some(result) = self.read_as_mut() {
            match result {
                Ok(inner) => inner.title(),
                Err(_) => "<ERRORED>",
            }
        } else {
            "<loading>"
        }
    }
}

struct StackBreadcrumb {
    title: String,
}

impl StackBreadcrumb {
    fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
        }
    }
}

impl Widget for StackBreadcrumb {
    fn ui(self, ui: &mut Ui) -> Response {
        TileFrame::default()
            .shape(TileFrameShape::Square)
            .bg_color(ui.style().visuals.faint_bg_color.gamma_multiply(6.0))
            .sense(Sense::click())
            .show(ui, |ui| {
                let text = RichText::new(self.title)
                    .heading();
                ui.add(Label::new(text).selectable(false));
            }).response
    }
}

enum MusicScreenStackEntry {
    Layer(Box<dyn MusicScreenLayer>),
    ScrollTo(ScrollToRequest),
}

pub struct MusicScreenStack {
    heos: Arc<HeosConnection<Stateful>>,
    layers: Vec<MusicScreenStackEntry>,
}

impl MusicScreenStack {
    pub fn new(heos: Arc<HeosConnection<Stateful>>) -> Self {
        Self {
            heos,
            layers: vec![],
        }
    }

    pub fn push(&mut self, layer: impl MusicScreenLayer + 'static) {
        self.layers.push(MusicScreenStackEntry::Layer(Box::new(layer)));
    }

    pub fn show(&mut self, ui: &mut Ui, actions: &mut Actions, playable_id: PlayableId) {
        ui.with_layout(Layout::left_to_right(Align::Min).with_main_wrap(false), |ui| {
            let mut clicked_idx = None;
            for (idx, layer) in self.layers.iter_mut().enumerate() {
                if idx > 0 {
                    ui.add(Label::new(RichText::new(" > ").heading()).selectable(false));
                }
                let title = match layer {
                    MusicScreenStackEntry::Layer(layer) => layer.title(),
                    MusicScreenStackEntry::ScrollTo(request) => request.breadcrumb.as_str(),
                };
                if ui.add(StackBreadcrumb::new(title)).clicked() {
                    clicked_idx = Some(idx);
                    break
                }
            }
            if let Some(clicked_idx) = clicked_idx {
                self.layers.truncate(clicked_idx + 1);

                let mut scroll_to_request = None;
                while let Some(entry) = self.layers.pop() {
                    match entry {
                        MusicScreenStackEntry::ScrollTo(request) => {
                            if scroll_to_request.is_none() {
                                scroll_to_request = Some(request)
                            }
                        },
                        layer => {
                            self.layers.push(layer);
                            break
                        }
                    }
                }
                if let Some(request) = scroll_to_request &&
                        let Some(entry) = self.layers.last_mut() &&
                        let MusicScreenStackEntry::Layer(layer) = entry {
                    // TODO: Always scrolling after adding a layer?
                    layer.scroll_to(request);
                }
            }
        });

        ui.add_space(8.0);

        if let Some(entry) = self.layers.last_mut() {
            match entry {
                MusicScreenStackEntry::Layer(layer) => {
                    layer.check_loaded();
                    let output = layer.show(ui);

                    for selection in output.selections {
                        match selection {
                            BrowseSelection::Play(play_selection) => match play_selection {
                                PlaySelection::PlayStation {
                                    source_id,
                                    parent_id,
                                    item,
                                } => {
                                    actions.play_station(
                                        playable_id,
                                        source_id,
                                        parent_id,
                                        item,
                                    );
                                },
                                PlaySelection::PlayInputSource {
                                    source_id,
                                    item,
                                } => {
                                    actions.play_input_source(
                                        playable_id,
                                        source_id,
                                        item,
                                    );
                                },
                                PlaySelection::AddToQueue {
                                    source_id,
                                    parent_id,
                                    item,
                                    operation,
                                } => {
                                    actions.add_to_queue(
                                        playable_id,
                                        source_id,
                                        parent_id,
                                        item,
                                        operation,
                                    );
                                },
                            },
                            BrowseSelection::Container(c_select) =>
                                self.layers.push(MusicScreenStackEntry::Layer(Box::new(BrowseContainer::new(
                                    self.heos.clone(),
                                    c_select.source_id,
                                    c_select.container,
                                )))),
                            BrowseSelection::ScrollTo(st_request) =>
                                self.layers.push(MusicScreenStackEntry::ScrollTo(st_request)),
                        }
                    }
                },
                MusicScreenStackEntry::ScrollTo { .. } => {
                    warn!("Tried to display a ScrollTo entry");
                }
            }
        }
    }
}

pub struct MusicScreen {
    search_text: String,
    stack: MusicScreenStack,
    playable_id: PlayableId,
}

impl MusicScreen {
    pub fn new(
        heos: Arc<HeosConnection<Stateful>>,
        stack: Option<MusicScreenStack>,
        playable_id: PlayableId,
    ) -> Self {
        let stack = stack.unwrap_or_else(|| {
            let mut stack = MusicScreenStack::new(heos.clone());
            let mut browse = Bind::new(true);
            browse.request(async move {
                Ok::<_, Infallible>(BrowseTop::init(heos).await)
            });
            stack.push(browse);
            stack
        });

        Self {
            search_text: String::default(),
            stack,
            playable_id,
        }
    }

    fn show(&mut self, ui: &mut Ui, actions: &mut Actions) {
        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
            let response = ui.add(
                TextEdit::singleline(&mut self.search_text)
                    .hint_text("🔍 search")
                    .char_limit(128)
                    //.background_color(ui.style().visuals.extreme_bg_color)
                    .font(FontSelection::Style(TextStyle::Heading))
            );

            ui.add_space(8.0);

            self.stack.show(ui, actions, self.playable_id);
        });
    }

    pub fn update(&mut self, ctx: &Context, actions: &mut Actions) {
        egui::CentralPanel::default().show(ctx, |ui| self.show(ui, actions));
    }

    pub fn into_stack(self) -> MusicScreenStack {
        self.stack
    }
}