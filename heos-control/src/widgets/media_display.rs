use eframe::epaint::text::TextWrapMode;
use egui::{Direction, FontSelection, Image, Label, Layout, Response, RichText, Ui, Widget, WidgetText};
use emath::Align;
use heos::data::media::MediaItem;

pub struct MediaDisplay<'a> {
    item: &'a MediaItem,
    dir: Option<Direction>,
    selectable: bool,
}

impl<'a> MediaDisplay<'a> {
    pub fn new(item: &'a MediaItem) -> Self {
        Self {
            item,
            dir: None,
            selectable: false,
        }
    }

    #[allow(unused)]
    pub fn direction(mut self, dir: Direction) -> Self {
        self.dir = Some(dir);
        self
    }

    #[allow(unused)]
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
}

impl<'a> Widget for MediaDisplay<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let dir = self.dir.unwrap_or(ui.layout().main_dir);

        let (outer_layout, align) = match dir {
            Direction::LeftToRight => (Layout::left_to_right(Align::Center), Align::LEFT),
            Direction::RightToLeft => (Layout::right_to_left(Align::Center), Align::RIGHT),
            // Assume default left-to-right layout
            Direction::TopDown | Direction::BottomUp => (Layout::left_to_right(Align::Center), Align::LEFT),
        };
        ui.with_layout(outer_layout, |ui| {
            if let Some(image_url) = self.item.image_url() {
                ui.add(Image::from_uri(image_url.as_str()));
            }

            ui.with_layout(Layout::top_down(align), |ui| {
                let lines = match self.item {
                    MediaItem::HeosService(heos_item) |
                    MediaItem::HeosServer(heos_item) => vec![
                        WidgetText::from(RichText::new(&heos_item.name).strong()),
                    ],
                    MediaItem::Artist(artist) => vec![
                        WidgetText::from(RichText::new(&artist.name).strong()),
                    ],
                    MediaItem::Album(album) => vec![
                        WidgetText::from(RichText::new(&album.name).strong()),
                        WidgetText::from(&album.artist),
                    ],
                    MediaItem::Song(song) => vec![
                        WidgetText::from(RichText::new(&song.name).strong()),
                        WidgetText::from(&song.album),
                        WidgetText::from(&song.artist),
                    ],
                    MediaItem::Container(container) => vec![
                        WidgetText::from(RichText::new(&container.name).strong()),
                    ],
                    MediaItem::Station(station) => vec![
                        WidgetText::from(RichText::new(&station.name).strong()),
                    ],
                    MediaItem::Genre(genre) => vec![
                        WidgetText::from(RichText::new(&genre.name).strong()),
                    ],
                };
                let galleys = lines.into_iter().map(|line| {
                    line.into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        ui.available_width(),
                        FontSelection::Default,
                    )
                }).collect::<Vec<_>>();
                let total_height = (ui.spacing().item_spacing.y * (galleys.len() - 1) as f32) +
                    galleys.iter().map(|galley| galley.rect.height()).sum::<f32>();
                let spacing = ui.available_height() - total_height;

                if spacing > 0.0 {
                    ui.add_space(spacing / 2.0);
                }
                for galley in galleys {
                    ui.add(Label::new(galley).selectable(self.selectable));
                }
            });
        }).response
    }
}