use egui::{Color32, Rect};

use crate::geometry::valid_rect;

use super::ExportError;

#[derive(Clone, Copy, Debug)]
pub(super) enum Background {
    Style,
    Transparent,
    Color(Color32),
}

/// Параметры экспорта; размеры штрихов и шрифтов заданы в логических точках.
#[derive(Clone, Debug)]
pub struct ExportOptions {
    pub(super) label_layout: Option<crate::labels::LabelLayoutOptions>,
    pub(super) scale: f32,
    pub(super) padding: f32,
    pub(super) region: Option<Rect>,
    pub(super) background: Background,
    pub(super) labels: bool,
    pub(super) loops: bool,
}

impl ExportOptions {
    /// Включает ту же раскладку, что используется на холсте; экспорт не сохраняет историю положений между вызовами.
    pub fn with_label_layout(
        mut self,
        options: Option<crate::labels::LabelLayoutOptions>,
    ) -> Result<Self, ExportError> {
        if options.is_some_and(|options| !options.valid()) {
            return Err(ExportError::InvalidOptions);
        }
        self.label_layout = options;
        Ok(self)
    }
    /// `scale` переводит мировые единицы в логические точки; `padding` добавляет поля вокруг всей схемы.
    pub fn new(scale: f32, padding: f32) -> Result<Self, ExportError> {
        if !scale.is_finite() || scale <= 0.0 || !padding.is_finite() || padding < 0.0 {
            return Err(ExportError::InvalidOptions);
        }
        Ok(Self {
            scale,
            padding,
            ..Self::default()
        })
    }

    /// Явная область в мировых координатах обрезает изображение; поля к ней не добавляются.
    pub fn with_region(mut self, region: Rect) -> Result<Self, ExportError> {
        if !valid_rect(region) || region.width() <= 0.0 || region.height() <= 0.0 {
            return Err(ExportError::InvalidOptions);
        }
        self.region = Some(region);
        Ok(self)
    }

    /// `None` оставляет фон прозрачным; по умолчанию используется фон `Style`.
    pub fn with_background(mut self, background: Option<Color32>) -> Self {
        self.background = background.map_or(Background::Transparent, Background::Color);
        self
    }

    pub fn with_labels(mut self, labels: bool) -> Self {
        self.labels = labels;
        self
    }

    pub fn with_loops(mut self, loops: bool) -> Self {
        self.loops = loops;
        self
    }
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            label_layout: None,
            scale: 1.0,
            padding: 16.0,
            region: None,
            background: Background::Style,
            labels: true,
            loops: true,
        }
    }
}
