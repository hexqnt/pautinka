use egui::{Pos2, Rect, Vec2};

use crate::{Camera, HitPoint, Node, NodeShape, SpatialError};

/// Локальные границы относительно позиции узла в мировых либо экранных координатах.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeGeometry {
    bounds: Rect,
    screen: bool,
}

impl NodeGeometry {
    pub(crate) const MIN_HIT_SIZE: f32 = 18.0;

    /// Границы в мировых единицах, локально относительно позиции узла.
    pub fn world(bounds: Rect) -> Result<Self, SpatialError> {
        Self::new(bounds, false)
    }
    /// Границы в экранных точках, локально относительно проекции позиции узла.
    pub fn screen(bounds: Rect) -> Result<Self, SpatialError> {
        Self::new(bounds, true)
    }
    fn new(bounds: Rect, screen: bool) -> Result<Self, SpatialError> {
        if !crate::geometry::valid_rect(bounds) {
            return Err(SpatialError::InvalidBounds);
        }
        Ok(Self { bounds, screen })
    }
    pub(crate) fn for_node(node: Node<'_>, shape: Option<NodeShape>) -> Self {
        Self {
            bounds: Rect::from_center_size(
                Pos2::ZERO,
                shape.map_or_else(|| Vec2::splat(node.radius * 2.0), NodeShape::size),
            ),
            screen: shape.is_none(),
        }
    }
    pub fn world_bounds(self, position: Pos2) -> Rect {
        if !self.bounds.is_finite() || self.bounds.width() < 0.0 || self.bounds.height() < 0.0 {
            return Rect::NOTHING;
        }
        if self.screen {
            Rect::from_min_max(position, position)
        } else {
            self.bounds.translate(position.to_vec2())
        }
    }
    pub fn screen_margin(self) -> f32 {
        if self.screen {
            self.bounds
                .min
                .to_vec2()
                .abs()
                .max(self.bounds.max.to_vec2().abs())
                .max_elem()
        } else {
            2.0
        }
    }
    pub(crate) fn focus_offset(self) -> Vec2 {
        if self.screen {
            self.bounds.center().to_vec2()
        } else {
            Vec2::ZERO
        }
    }
    pub fn screen_rect(self, position: Pos2, camera: Camera, viewport: Rect) -> Rect {
        let center = camera.to_screen(position, viewport);
        if self.screen {
            self.bounds.translate(center.to_vec2())
        } else {
            let rect = Rect::from_min_max(
                center + self.bounds.min.to_vec2() * camera.zoom,
                center + self.bounds.max.to_vec2() * camera.zoom,
            );
            Rect::from_center_size(rect.center(), rect.size().max(Vec2::splat(4.0)))
        }
    }
    pub(crate) fn hit_test_shape(
        self,
        position: Pos2,
        point: HitPoint,
        zoom: f32,
        shape: NodeShape,
    ) -> bool {
        // Малые фигуры сохраняют свою форму в области выбора размером не менее 18 экранных точек.
        let hit_size = self
            .bounds
            .size()
            .max(Vec2::splat(Self::MIN_HIT_SIZE / zoom));
        let relative = point.relative_to(position);
        shape.contains_local(relative / hit_size * shape.size())
    }
    pub fn hit_test(self, position: Pos2, point: Pos2, zoom: f32) -> bool {
        HitPoint::from_world(point)
            .is_some_and(|point| self.hit_test_precise(position, point, zoom))
    }
    pub fn hit_test_precise(self, position: Pos2, point: HitPoint, zoom: f32) -> bool {
        let (point, minimum) = if self.screen {
            (point.relative_to(position) * zoom, Self::MIN_HIT_SIZE)
        } else {
            (point.relative_to(position), Self::MIN_HIT_SIZE / zoom)
        };
        Rect::from_center_size(
            self.bounds.center(),
            self.bounds.size().max(Vec2::splat(minimum)),
        )
        .contains(point.to_pos2())
    }
}
