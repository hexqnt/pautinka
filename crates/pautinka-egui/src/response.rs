use std::{error::Error, fmt, ops::Deref};

use egui::{Pos2, Rect, Response};

use crate::{ElementId, SpatialError, geometry::Camera, options::DetailLevel};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CanvasError {
    Spatial(SpatialError),
    InvalidCamera,
    InvalidOptions,
    InvalidGraphBounds,
}

impl From<SpatialError> for CanvasError {
    fn from(value: SpatialError) -> Self {
        Self::Spatial(value)
    }
}
impl Error for CanvasError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Spatial(error) => Some(error),
            _ => None,
        }
    }
}

impl fmt::Display for CanvasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spatial(error) => error.fmt(f),
            Self::InvalidCamera => f.write_str("Camera must have a finite center, positive finite zoom and representable transforms"),
            Self::InvalidOptions => f.write_str(
                "Canvas margins, focus scale and wheel sensitivity must be finite and valid",
            ),
            Self::InvalidGraphBounds => f.write_str("Graph bounds must be finite and ordered"),
        }
    }
}
/// Данные последнего показа не заимствуют модель; события остаются в переиспользуемом буфере `Canvas::events`.
pub struct CanvasResponse {
    pub response: Response,
    pub viewport: Rect,
    pub camera: Camera,
    pub hovered: Option<ElementId>,
    pub context_target: Option<ElementId>,
    pub context_position: Option<Pos2>,
    pub detail: DetailLevel,
    pub selection_changed: bool,
    pub camera_changed: bool,
    /// Устанавливается только удобным `show_with`; `try_show_with` возвращает `Err`.
    pub error: Option<CanvasError>,
}

impl CanvasResponse {
    pub fn to_world_precise(&self, screen: Pos2) -> Option<crate::HitPoint> {
        self.camera.to_world_precise(screen, self.viewport)
    }
    pub fn to_world(&self, screen: Pos2) -> Pos2 {
        self.camera.to_world(screen, self.viewport)
    }
    pub fn to_screen(&self, world: Pos2) -> Pos2 {
        self.camera.to_screen(world, self.viewport)
    }
}
impl Deref for CanvasResponse {
    type Target = Response;
    fn deref(&self) -> &Response {
        &self.response
    }
}
