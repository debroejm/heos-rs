pub mod frame;
pub mod media;

use egui::{Direction, Frame, Image, Label, Layout, Response, RichText, TextStyle, Ui, Widget};
use std::fmt::Display;
use eframe::epaint::Color32;
use emath::Align;
use url::Url;
use heos::data::source::{SourceInfo, SourceType};

pub struct ErrorSplash<'a, E> {
    error: &'a E,
    layout: Option<Layout>,
}

impl<'a, E> ErrorSplash<'a, E> {
    pub fn new(error: &'a E) -> Self {
        Self {
            error,
            layout: None,
        }
    }

    #[allow(dead_code)]
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn centered(mut self) -> Self {
        self.layout = Some(Layout::centered_and_justified(Direction::TopDown));
        self
    }
}

impl<'a, E> Widget for ErrorSplash<'a, E>
where
    E: Display
{
    fn ui(self, ui: &mut Ui) -> Response {
        let layout = self.layout
            .unwrap_or_else(|| ui.layout().clone());

        let inner = ui.with_layout(layout, |ui| {
            let text = RichText::new(self.error.to_string())
                .heading()
                .color(ui.visuals().error_fg_color);
            ui.add(Label::new(text))
        });
        inner.response | inner.inner
    }
}

enum SectionHeaderContent {
    Text(String),
    Image(Url),
}

pub struct SectionHeader {
    content: SectionHeaderContent,
    text_style: Option<TextStyle>,
    bg_color: Option<Color32>,
    height: Option<f32>,
}

impl SectionHeader {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: SectionHeaderContent::Text(text.into()),
            text_style: None,
            bg_color: None,
            height: None,
        }
    }

    pub fn image(url: impl Into<Url>) -> Self {
        Self {
            content: SectionHeaderContent::Image(url.into()),
            text_style: None,
            bg_color: None,
            height: None,
        }
    }

    pub fn from_source(info: &SourceInfo) -> Self {
        match info.source_type {
            SourceType::MusicService => Self::image(info.image_url.clone()),
            // HEOS-provided images for non-music-service sources aren't very good;
            // just use the name instead
            _ => Self::text(info.name.clone()),
        }
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn bg_color(mut self, bg_color: impl Into<Color32>) -> Self {
        self.bg_color = Some(bg_color.into());
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl Widget for SectionHeader {
    fn ui(self, ui: &mut Ui) -> Response {
        let text_style = self.text_style
            .or(ui.style().override_text_style.clone())
            .unwrap_or(TextStyle::Heading);
        let bg_color = self.bg_color
            .unwrap_or_else(|| ui.visuals().faint_bg_color);
        let height = self.height
            .unwrap_or_else(|| ui.text_style_height(&text_style));

        let mut frame = Frame::NONE
            .inner_margin(4.0)
            .corner_radius(0.0)
            .fill(bg_color)
            .begin(ui);

        let inner = frame.content_ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(height);

            match self.content {
                SectionHeaderContent::Text(text) => {
                    // Layout::centered_and_justified takes more space than requested, so do our
                    // own centering instead
                    let spacing = (height - ui.text_style_height(&text_style)) / 2.0;
                    if spacing > 0.0 {
                        ui.add_space(spacing);
                    }
                    ui.label(RichText::new(text)
                        .strong()
                        .text_style(text_style))
                },
                SectionHeaderContent::Image(url) => {
                    ui.add(Image::from_uri(url.as_str()))
                },
            }
        });

        let response = frame.allocate_space(ui);
        frame.paint(ui);
        response | inner.response
    }
}