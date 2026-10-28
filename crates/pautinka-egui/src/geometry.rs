use egui::{Pos2, Rect, Vec2};

use crate::HitPoint;

pub use pautinka_core::{Curve, EdgePath, PathError, PathSample};

/// Преобразование между координатами графа и экраном.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub center: Pos2,
    pub zoom: f32,
    pub fit: bool,
}

impl Camera {
    pub fn fit(&mut self, bounds: Rect, viewport: Rect) {
        self.center = if bounds.is_finite() {
            Pos2::new(
                bounds.min.x * 0.5 + bounds.max.x * 0.5,
                bounds.min.y * 0.5 + bounds.max.y * 0.5,
            )
        } else {
            Pos2::ZERO
        };
        self.zoom = Self::fit_zoom(bounds, viewport);
        self.fit = false;
    }

    /// Минимальный масштаб, при котором границы графа помещаются в область просмотра.
    pub fn fit_zoom(bounds: Rect, viewport: Rect) -> f32 {
        if !bounds.is_finite() || bounds.size() == Vec2::ZERO {
            return 20.0;
        }
        let horizontal = if bounds.width() > 0.0 {
            viewport.width().max(1.0) / bounds.width()
        } else {
            f32::MAX
        };
        let vertical = if bounds.height() > 0.0 {
            viewport.height().max(1.0) / bounds.height()
        } else {
            f32::MAX
        };
        horizontal.min(vertical).clamp(f32::MIN_POSITIVE, f32::MAX)
    }

    pub fn to_screen(self, point: Pos2, viewport: Rect) -> Pos2 {
        viewport.center() + (point - self.center) * self.zoom
    }

    pub fn to_world(self, point: Pos2, viewport: Rect) -> Pos2 {
        self.center + (point - viewport.center()) / self.zoom
    }

    pub(crate) fn rect_to_world(self, rect: Rect, viewport: Rect) -> Rect {
        Rect::from_min_max(
            self.to_world(rect.min, viewport),
            self.to_world(rect.max, viewport),
        )
    }

    /// Преобразование указателя без округления малых смещений относительно большой мировой позиции.
    pub fn to_world_precise(self, point: Pos2, viewport: Rect) -> Option<HitPoint> {
        let center = viewport.center();
        HitPoint::new(
            f64::from(self.center.x)
                + (f64::from(point.x) - f64::from(center.x)) / f64::from(self.zoom),
            f64::from(self.center.y)
                + (f64::from(point.y) - f64::from(center.y)) / f64::from(self.zoom),
        )
    }

    pub fn zoom_at(&mut self, point: Pos2, factor: f32, viewport: Rect) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        self.set_zoom_at(
            point,
            (self.zoom * factor).clamp(f32::MIN_POSITIVE, f32::MAX),
            viewport,
        );
    }

    pub fn set_zoom_at(&mut self, point: Pos2, zoom: f32, viewport: Rect) {
        if !zoom.is_finite() || zoom <= 0.0 {
            return;
        }
        let anchor = self.to_world(point, viewport);
        self.zoom = zoom;
        self.center = anchor - (point - viewport.center()) / self.zoom;
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            center: Pos2::ZERO,
            zoom: 1.0,
            fit: true,
        }
    }
}

pub(crate) fn valid_rect(bounds: Rect) -> bool {
    bounds.is_finite()
        && bounds.size().is_finite()
        && bounds.width() >= 0.0
        && bounds.height() >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    #[test]
    fn zoom_keeps_pointer_anchor_and_roundtrips_coordinates() {
        let viewport = Rect::from_min_size(pos2(100.0, 80.0), Vec2::new(800.0, 500.0));
        let mut camera = Camera::default();
        camera.fit(
            Rect::from_min_max(pos2(-300.0, -100.0), pos2(500.0, 200.0)),
            viewport,
        );
        let pointer = pos2(234.0, 170.0);
        let anchor = camera.to_world(pointer, viewport);
        camera.zoom_at(pointer, 2.0, viewport);
        assert!(camera.to_screen(anchor, viewport).distance(pointer) < 0.001);
        assert!(
            camera
                .to_world(camera.to_screen(anchor, viewport), viewport)
                .distance(anchor)
                < 0.001
        );
    }
    #[test]
    fn fit_zoom_handles_large_and_degenerate_bounds() {
        let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 500.0));
        let bounds = Rect::from_center_size(Pos2::ZERO, Vec2::splat(1_000_000.0));
        let mut camera = Camera::default();
        camera.fit(bounds, viewport);
        assert_eq!(camera.zoom, 0.0005);
        assert!(
            viewport
                .expand(0.001)
                .contains(camera.to_screen(bounds.min, viewport))
        );
        assert!(
            viewport
                .expand(0.001)
                .contains(camera.to_screen(bounds.max, viewport))
        );
        assert_eq!(Camera::fit_zoom(Rect::NOTHING, viewport), 20.0);
        assert_eq!(Camera::fit_zoom(Rect::ZERO, viewport), 20.0);
        assert!(Camera::fit_zoom(bounds, Rect::ZERO) > 0.0);
    }
}
