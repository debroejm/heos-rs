use eframe::epaint::text::TextWrapMode;
use egui::{Direction, FontSelection, Image, Label, Layout, Response, RichText, Ui, Widget, WidgetText};
use emath::Align;
use heos::data::song::SongInfo;
use std::sync::Arc;

pub struct MediaDisplay<'a> {
    info: &'a SongInfo,
    dir: Option<Direction>,
    selectable: bool,
}

impl<'a> MediaDisplay<'a> {
    pub fn new(info: &'a SongInfo) -> Self {
        Self {
            info,
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
            if let Some(image_url) = &self.info.image_url {
                ui.add(Image::from_uri(image_url.as_str()));
            }

            ui.with_layout(Layout::top_down(align), |ui| {
                let lines = [
                    WidgetText::RichText(Arc::new(RichText::new(&self.info.song).strong())),
                    WidgetText::Text(self.info.album.clone()),
                    WidgetText::Text(self.info.artist.clone()),
                ];
                let galleys = lines.map(|line| {
                    line.into_galley(
                        ui,
                        Some(TextWrapMode::Truncate),
                        ui.available_width(),
                        FontSelection::Default,
                    )
                });
                let total_height = (ui.spacing().item_spacing.y * 2.0) +
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