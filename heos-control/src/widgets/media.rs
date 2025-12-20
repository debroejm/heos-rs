use bitflags::bitflags;
use eframe::epaint::text::TextWrapMode;
use egui::{Button, Direction, FontSelection, Frame, Galley, Grid, Image, Label, Layout, Margin, Popup, Response, RichText, ScrollArea, Sense, TextStyle, Ui, Widget, WidgetText};
use emath::Align;
use heos::data::media::{MediaContainerRef, MediaItemBase, MediaItemType};
use heos::data::player::AddToQueueType;
use heos::data::source::SourceId;
use std::sync::Arc;
use strum::IntoDiscriminant;

use crate::assets;
use crate::screen::music::{BrowseSelection, PlaySelection};
use crate::widgets::frame::{TileFrame, TileFrameShape};

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MediaDisplaySense(u8);

bitflags! {
    impl MediaDisplaySense: u8 {
        const IMAGE = 1<<0;
        const NAME = 1<<1;
        const ALBUM = 1<<2;
        const ARTIST = 1<<3;
    }
}

#[derive(Clone, Debug)]
#[allow(unused)]
pub struct MediaDisplayOutput {
    pub bg_response: Response,
    pub image: Option<Response>,
    pub name: Response,
    pub album: Option<Response>,
    pub artist: Option<Response>,
}

pub struct MediaDisplay<'a, T: MediaItemBase> {
    item: &'a T,
    dir: Option<Direction>,
    selectable: bool,
    sense: MediaDisplaySense,
}

impl<'a, T: MediaItemBase> MediaDisplay<'a, T> {
    pub fn new(item: &'a T) -> Self {
        Self {
            item,
            dir: None,
            selectable: false,
            sense: MediaDisplaySense::empty(),
        }
    }

    pub fn direction(mut self, dir: Direction) -> Self {
        self.dir = Some(dir);
        self
    }

    #[allow(unused)]
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    pub fn sense(mut self, sense: MediaDisplaySense) -> Self {
        self.sense = sense;
        self
    }

    pub fn show(self, ui: &mut Ui) -> MediaDisplayOutput {
        let dir = self.dir.unwrap_or(ui.layout().main_dir);

        let (outer_layout, align) = match dir {
            Direction::LeftToRight => (Layout::left_to_right(Align::Center), Align::LEFT),
            Direction::RightToLeft => (Layout::right_to_left(Align::Center), Align::RIGHT),
            Direction::TopDown => (Layout::top_down(Align::Center), Align::LEFT),
            Direction::BottomUp => (Layout::bottom_up(Align::Center), Align::LEFT),
        };
        let inner = ui.with_layout(outer_layout, |ui| {
            let text_fn = |ui: &mut Ui, wrap_mode: TextWrapMode, font_selection: FontSelection| {
                let create_galley = |text: WidgetText| text.into_galley(
                    ui,
                    Some(wrap_mode),
                    ui.available_width(),
                    font_selection.clone(),
                );

                let name = create_galley(WidgetText::from(RichText::new(self.item.name()).strong()));
                let album = self.item.album().map(|album| create_galley(WidgetText::from(album)));
                let artist = self.item.artist().map(|artist| create_galley(WidgetText::from(artist)));

                let total_height = std::iter::once(Some(&name))
                    .chain([album.as_ref(), artist.as_ref()])
                    .filter_map(std::convert::identity)
                    .enumerate()
                    .map(|(idx, galley)| {
                        if idx > 0 {
                            ui.spacing().item_spacing.y + galley.rect.height()
                        } else {
                            galley.rect.height()
                        }
                    })
                    .sum::<f32>();
                let spacing = ui.available_height() - total_height;

                if spacing > 0.0 {
                    ui.add_space(spacing / 2.0);
                }

                let mut add_label = |galley: Arc<Galley>, sense: MediaDisplaySense| {
                    if self.sense.contains(sense) {
                        ui.add(Label::new(galley).selectable(self.selectable).sense(Sense::click()))
                    } else {
                        ui.add(Label::new(galley).selectable(self.selectable))
                    }
                };

                (
                    add_label(name, MediaDisplaySense::NAME),
                    album.map(|album| add_label(album, MediaDisplaySense::ALBUM)),
                    artist.map(|artist| add_label(artist, MediaDisplaySense::ARTIST)),
                )
            };

            if let Some(image_url) = self.item.image_url() {
                let image = ui.add(Image::from_uri(image_url.as_str()));
                let (name, album, artist) = ui.with_layout(
                    Layout::top_down_justified(align),
                    |ui| text_fn(ui, TextWrapMode::Truncate, FontSelection::Default),
                ).inner;
                (Some(image), name, album, artist)
            } else {
                let (name, album, artist) = ui.with_layout(
                    Layout::top_down_justified(Align::Center),
                    |ui| text_fn(ui, TextWrapMode::Wrap, FontSelection::Style(TextStyle::Heading)),
                ).inner;
                (None, name, album, artist)
            }
        });

        let (image, name, album, artist) = inner.inner;
        MediaDisplayOutput {
            bg_response: inner.response,
            image,
            name,
            album,
            artist
        }
    }
}

impl<'a, T: MediaItemBase> Widget for MediaDisplay<'a, T> {
    #[inline]
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).bg_response
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MediaItemSelection<'a, T: MediaItemBase> {
    pub item: &'a T,
    pub operation: AddToQueueType,
}

impl<'a, T: MediaItemBase> MediaItemSelection<'a, T> {
    #[inline]
    pub fn into_browse_selection(self, source_id: SourceId, parent_id: Option<String>) -> BrowseSelection {
        BrowseSelection::Play(PlaySelection::AddToQueue {
            source_id,
            parent_id,
            item: self.item.as_media_item_ref().into_media_item(),
            operation: self.operation,
        })
    }
}

#[derive(Debug, Clone)]
pub enum MediaTileSelection<'a, T: MediaItemBase> {
    PlayStation(&'a T),
    PlayInputSource(&'a T),
    AddToQueue(MediaItemSelection<'a, T>),
    GoToContainer(MediaContainerRef<'a>),
    SeeMore,
}

#[derive(Clone, Copy)]
enum MediaTileType<'a, T: MediaItemBase> {
    Item(&'a T),
    SeeMore(usize),
}

pub struct MediaTile<'a, T: MediaItemBase> {
    inner: MediaTileType<'a, T>,
    size: Option<f32>,
    frame: TileFrame,
}

impl<'a, T: MediaItemBase> MediaTile<'a, T> {
    pub const DEFAULT_SIZE: f32 = 200.0;

    pub fn new(item: &'a T) -> Self {
        Self {
            inner: MediaTileType::Item(item),
            size: None,
            frame: TileFrame::default().sense(Sense::click()),
        }
    }

    pub fn see_more(more_count: usize) -> Self {
        Self {
            inner: MediaTileType::SeeMore(more_count),
            size: None,
            frame: TileFrame::default().sense(Sense::click()),
        }
    }

    #[allow(dead_code)]
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn get_width(&self) -> f32 {
        let margin = self.frame.get_inner_margin();
        self.size.unwrap_or(Self::DEFAULT_SIZE) + margin.leftf() + margin.rightf()
    }

    pub fn show(self, ui: &mut Ui) -> Option<MediaTileSelection<'a, T>> {
        let size = self.size.unwrap_or(Self::DEFAULT_SIZE);
        let inner_response = self.frame.show(ui, |ui| {
            ui.set_min_size(egui::vec2(size, size));
            ui.set_max_size(egui::vec2(size, size));

            match self.inner {
                MediaTileType::Item(item) => {
                    let output = MediaDisplay::new(item)
                        .direction(Direction::TopDown)
                        .sense(MediaDisplaySense::NAME)
                        .show(ui);
                    Some(output)
                },
                MediaTileType::SeeMore(more_count) => {
                    ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                        let text = RichText::new(format!("See {more_count} more"))
                            .heading()
                            .strong();
                        ui.add(Label::new(text).selectable(false));
                    });
                    None
                }
            }
        });

        let mut selection = None;
        match self.inner {
            MediaTileType::Item(item) => {
                let output = inner_response.inner
                    .expect("output should not be None");
                let bg_response = inner_response.response | output.bg_response;

                if output.name.clicked() || bg_response.clicked() {
                    selection = Some(match item.try_as_media_container_ref() {
                        Some(container) => MediaTileSelection::GoToContainer(container),
                        None => {
                            match item.as_media_item_ref().discriminant() {
                                MediaItemType::HeosService |
                                MediaItemType::HeosServer => MediaTileSelection::PlayInputSource(item),
                                MediaItemType::Artist |
                                MediaItemType::Album |
                                MediaItemType::Song |
                                MediaItemType::Container => MediaTileSelection::AddToQueue(MediaItemSelection {
                                    item,
                                    operation: AddToQueueType::PlayNow,
                                }),
                                MediaItemType::Station |
                                MediaItemType::Genre => MediaTileSelection::PlayStation(item),
                            }
                        },
                    });
                }

                Popup::context_menu(&bg_response).show(|ui| {
                    if let Some(container) = item.try_as_media_container_ref() {
                        if ui.button(format!("View {}", container.name())).clicked() {
                            selection = Some(MediaTileSelection::GoToContainer(container));
                        }
                    }

                    match item.as_media_item_ref().discriminant() {
                        MediaItemType::HeosService |
                        MediaItemType::HeosServer => {
                            if ui.button(format!("Play from {}", item.name())).clicked() {
                                selection = Some(MediaTileSelection::PlayInputSource(item));
                            }
                        },
                        MediaItemType::Artist |
                        MediaItemType::Album |
                        MediaItemType::Song |
                        MediaItemType::Container => {
                            if item.playable() {
                                if ui.button("Play Now").clicked() {
                                    selection = Some(MediaTileSelection::AddToQueue(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::PlayNow,
                                    }));
                                }
                                if ui.button("Play Next").clicked() {
                                    selection = Some(MediaTileSelection::AddToQueue(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::PlayNext,
                                    }));
                                }
                                if ui.button("Add to Queue").clicked() {
                                    selection = Some(MediaTileSelection::AddToQueue(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::AddToEnd,
                                    }));
                                }
                                if ui.button("Play Now and Replace Queue").clicked() {
                                    selection = Some(MediaTileSelection::AddToQueue(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::ReplaceAndPlay,
                                    }));
                                }
                            }
                        },
                        MediaItemType::Station |
                        MediaItemType::Genre => {
                            if ui.button(format!("Play {}", item.name())).clicked() {
                                selection = Some(MediaTileSelection::PlayStation(item));
                            }
                        }
                    }
                });
            },
            MediaTileType::SeeMore(_) => {
                if inner_response.response.clicked() {
                    selection = Some(MediaTileSelection::SeeMore);
                }
            }
        }
        selection
    }
}

#[derive(Debug)]
pub struct MediaCarousalOutput<'a, T: MediaItemBase> {
    pub selection: Option<MediaTileSelection<'a, T>>,
}

pub struct MediaCarousal<'a, T: MediaItemBase> {
    items: &'a Vec<T>,
    count: usize,
}

impl<'a, T: MediaItemBase> MediaCarousal<'a, T> {
    pub fn new(items: &'a Vec<T>) -> Self {
        Self {
            items,
            count: items.len(),
        }
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
}

impl<'a, T> MediaCarousal<'a, T>
where
    T: MediaItemBase + Clone,
{
    pub fn show(self, ui: &mut Ui) -> MediaCarousalOutput<'a, T> {
        let mut selection = None;

        Frame::central_panel(ui.style()).fill(ui.style().visuals.extreme_bg_color).show(ui, |ui| {
            ScrollArea::horizontal().auto_shrink(emath::Vec2b::new(false, true)).show(ui, |ui| {
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    for item in self.items {
                        if let Some(tile_select) = MediaTile::new(item).show(ui) {
                            selection = Some(tile_select);
                        }
                    }

                    if self.count > self.items.len() {
                        if let Some(tile_select) = MediaTile::<T>::see_more(self.count - self.items.len()).show(ui) {
                            selection = Some(tile_select);
                        }
                    }
                });
            });
        });

        MediaCarousalOutput {
            selection,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaGalleryOutput<'a, T: MediaItemBase> {
    pub selection: Option<MediaTileSelection<'a, T>>,
}

pub struct MediaGallery<'a, T: MediaItemBase> {
    items: &'a Vec<T>,
}

impl<'a, T: MediaItemBase> MediaGallery<'a, T> {
    pub fn new(items: &'a Vec<T>) -> Self {
        Self {
            items,
        }
    }

    pub fn show(self, ui: &mut Ui) -> MediaGalleryOutput<'a, T> {
        let mut selection = None;

        Frame::central_panel(ui.style()).fill(ui.style().visuals.extreme_bg_color).show(ui, |ui| {
            ui.set_width(ui.available_width());

            ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                let mut iter = self.items.iter()
                    .map(MediaTile::new)
                    .peekable();
                while iter.peek().is_some() {
                    ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                        while let Some(tile_peek) = iter.peek() {
                            if tile_peek.get_width() < ui.available_width() {
                                if let Some(tile_select) = iter.next().unwrap().show(ui) {
                                    selection = Some(tile_select);
                                }
                            } else {
                                break
                            }
                        }
                    });
                }
            });
        });

        MediaGalleryOutput {
            selection,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaListOutput<'a, T: MediaItemBase> {
    pub selection: Option<MediaItemSelection<'a, T>>,
}

pub struct MediaList<'a, T: MediaItemBase> {
    items: &'a Vec<T>,
}

impl<'a, T: MediaItemBase> MediaList<'a, T> {
    pub fn new(items: &'a Vec<T>) -> Self {
        Self {
            items,
        }
    }

    pub fn show(self, ui: &mut Ui) -> MediaListOutput<'a, T> {
        let mut selection = None;

        let grid = Grid::new("media-list")
            .spacing(emath::vec2(0.0, 0.0))
            .num_columns(1);
        grid.show(ui, |ui| {
            for (idx, item) in self.items.iter().enumerate() {
                let frame = TileFrame::default()
                    .shape(TileFrameShape::Square)
                    .bg_color(match idx % 2 == 0 {
                        true => ui.style().visuals.faint_bg_color.gamma_multiply(2.0),
                        false => ui.style().visuals.faint_bg_color.gamma_multiply(4.0),
                    })
                    .inner_margin(Margin {
                        left: 32,
                        right: 32,
                        top: 8,
                        bottom: 8,
                    });
                frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // The '0.0' spacing is required to prevent .columns() from panicking due to a
                    // negative width calculation when .available_width() is '0.0'
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.columns_const(|[ui_left, ui_center, ui_right]| {
                        let name = WidgetText::from(
                            RichText::new(item.name())
                                .heading()
                                .strong()
                        ).into_galley(
                            ui_left,
                            Some(TextWrapMode::Truncate),
                            ui_left.available_width(),
                            FontSelection::Style(TextStyle::Heading),
                        );
                        let text_height = name.rect.height();
                        ui_left.add(Label::new(name).selectable(true).truncate());

                        if let Some(artist) = item.artist() {
                            ui_center.add(Label::new(
                                RichText::new(artist)
                                    .heading()
                            ).selectable(true).truncate());
                        }

                        ui_right.with_layout(Layout::right_to_left(Align::Center), |ui_right| {
                            let play_image = assets::icons::play_small::image()
                                .fit_to_exact_size(emath::vec2(text_height, text_height));
                            let play_button = Button::image(play_image)
                                .frame(false)
                                .min_size(emath::vec2(text_height, text_height))
                                .image_tint_follows_text_color(true);
                            let play_response = ui_right.add(play_button);
                            if play_response.clicked() {
                                selection = Some(MediaItemSelection {
                                    item,
                                    operation: AddToQueueType::PlayNow,
                                });
                            }
                            Popup::context_menu(&play_response).show(|ui| {
                                if ui.button("Play Now").clicked() {
                                    selection = Some(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::PlayNow,
                                    });
                                }
                                if ui.button("Play Now and Replace Queue").clicked() {
                                    selection = Some(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::ReplaceAndPlay,
                                    });
                                }
                            });

                            ui_right.add_space(16.0);

                            let plus_image = assets::icons::plus::image()
                                .fit_to_exact_size(emath::vec2(text_height, text_height));
                            let plus_button = Button::image(plus_image)
                                .frame(false)
                                .min_size(emath::vec2(text_height, text_height))
                                .image_tint_follows_text_color(true);
                            let plus_response = ui_right.add(plus_button);
                            if plus_response.clicked() {
                                selection = Some(MediaItemSelection {
                                    item,
                                    operation: AddToQueueType::AddToEnd,
                                });
                            }
                            Popup::context_menu(&plus_response).show(|ui| {
                                if ui.button("Add Next").clicked() {
                                    selection = Some(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::PlayNext,
                                    });
                                }
                                if ui.button("Add to End of Queue").clicked() {
                                    selection = Some(MediaItemSelection {
                                        item,
                                        operation: AddToQueueType::AddToEnd,
                                    });
                                }
                            });
                        });
                    });
                });
                ui.end_row();
            }
        });

        MediaListOutput {
            selection,
        }
    }
}