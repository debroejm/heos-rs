use egui::{Color32, Frame, InnerResponse, Margin, Sense, Ui, UiBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileFrameShape {
    Square,
    Rounded,
}

impl Default for TileFrameShape {
    #[inline]
    fn default() -> Self {
        Self::Rounded
    }
}

impl TileFrameShape {
    pub fn corner_radius(&self) -> f32 {
        match self {
            Self::Square => 0.0,
            Self::Rounded => 6.0,
        }
    }

    pub fn inner_margin(&self) -> Margin {
        match self {
            Self::Square => 4.0.into(),
            Self::Rounded => 8.0.into(),
        }
    }
}

#[derive(Default)]
pub struct TileFrame {
    shape: Option<TileFrameShape>,
    inner_margin: Option<Margin>,
    sense: Option<Sense>,
    bg_color: Option<Color32>,
    hovered_color: Option<Color32>,
}

impl TileFrame {
    pub fn shape(mut self, shape: TileFrameShape) -> Self {
        self.shape = Some(shape);
        self
    }

    pub fn get_shape(&self) -> TileFrameShape {
        self.shape.unwrap_or_default()
    }

    pub fn inner_margin(mut self, inner_margin: impl Into<Margin>) -> Self {
        self.inner_margin = Some(inner_margin.into());
        self
    }

    pub fn get_inner_margin(&self) -> Margin {
        self.inner_margin.unwrap_or_else(|| self.get_shape().inner_margin())
    }

    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = Some(sense);
        self
    }

    pub fn bg_color(mut self, color: impl Into<Color32>) -> Self {
        self.bg_color = Some(color.into());
        self
    }

    #[allow(dead_code)]
    pub fn hovered_color(mut self, color: impl Into<Color32>) -> Self {
        self.hovered_color = Some(color.into());
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let shape = self.get_shape();
        let inner_margin = self.get_inner_margin();
        let sense_hover = self.sense
            .map(|sense| sense.contains(Sense::hover()))
            .unwrap_or(false);

        let mut frame = Frame::NONE
            .inner_margin(inner_margin)
            .corner_radius(shape.corner_radius())
            .begin(ui);

        let mut ui_builder = UiBuilder::new();
        ui_builder.sense = self.sense;

        let mut inner_response = frame.content_ui.scope_builder(ui_builder, add_contents);

        let response = frame.allocate_space(ui);
        if sense_hover && response.hovered() {
            frame.frame.fill = self.hovered_color
                .unwrap_or_else(|| ui.style().visuals.selection.bg_fill);
        } else {
            frame.frame.fill = self.bg_color
                .unwrap_or_else(|| ui.style().visuals.faint_bg_color.gamma_multiply(2.0));
        }
        frame.paint(ui);

        inner_response.response |= response;
        inner_response
    }
}