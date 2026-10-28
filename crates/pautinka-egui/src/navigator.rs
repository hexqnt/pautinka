mod cache;

pub(super) use cache::NavigatorCache;

use egui::{
    Align2, Color32, Painter, Pos2, Rect, Role, Sense, Shape, Stroke, StrokeKind, Ui, Vec2,
    WidgetInfo,
    epaint::{CubicBezierShape, QuadraticBezierShape},
};

use crate::{Camera, Canvas, Graph, geometry::Curve};

const PANEL_SIZE: Vec2 = Vec2::new(206.0, 161.0);
const OUTER_MARGIN: f32 = 12.0;
const INNER_MARGIN: f32 = 8.0;
const GRAPH_MARGIN: f32 = 3.0;
const MIN_PANEL_SIZE: f32 = 32.0;
const MIN_MARKER_SIZE: f32 = 4.0;

/// Угол холста для размещения мини-карты.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NavigatorCorner {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

impl NavigatorCorner {
    fn anchor(self) -> Align2 {
        match self {
            Self::TopLeft => Align2::LEFT_TOP,
            Self::TopRight => Align2::RIGHT_TOP,
            Self::BottomLeft => Align2::LEFT_BOTTOM,
            Self::BottomRight => Align2::RIGHT_BOTTOM,
        }
    }
}

/// Мини-карта выключена по умолчанию; включённая остаётся видимой при любом масштабе.
#[derive(Clone, Copy, Debug, Default)]
pub struct NavigatorOptions {
    pub enabled: bool,
    pub corner: NavigatorCorner,
}

pub(super) struct Navigator {
    panel: Rect,
    thumbnail: Rect,
    camera: Camera,
    bounds: Rect,
}

impl Navigator {
    pub fn new(viewport: Rect, bounds: Rect, corner: NavigatorCorner) -> Option<Self> {
        let available = viewport.shrink(OUTER_MARGIN);
        let size = PANEL_SIZE.min(available.size());
        if size.min_elem() < MIN_PANEL_SIZE {
            return None;
        }
        let panel = corner.anchor().align_size_within_rect(size, available);
        let thumbnail = panel.shrink(INNER_MARGIN);
        let mut camera = Camera::default();
        camera.fit(bounds, thumbnail.shrink(GRAPH_MARGIN));
        Some(Self {
            panel,
            thumbnail,
            camera,
            bounds,
        })
    }

    pub fn blocks_pointer(&self, ui: &Ui) -> bool {
        ui.input(|input| {
            input
                .pointer
                .hover_pos()
                .is_some_and(|p| self.panel.contains(p))
                || (input.pointer.any_down()
                    && input
                        .pointer
                        .press_origin()
                        .is_some_and(|p| self.panel.contains(p)))
        })
    }

    pub fn interact(&self, ui: &Ui, pan: bool) -> Option<Pos2> {
        let response = ui.interact(
            self.panel,
            ui.scope_id().with("navigator"),
            if pan {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            },
        );
        response.widget_info(|| {
            WidgetInfo::labeled(Role::Unknown, ui.is_enabled() && pan, "Navigator")
        });
        let target = if response.clicked() || response.dragged() {
            response.interact_pointer_pos().map(|p| self.target(p))
        } else {
            None
        };
        let response = response.on_hover_text("Navigator: click or drag to pan the viewport");
        if pan {
            response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        target
    }

    fn target(&self, point: Pos2) -> Pos2 {
        self.bounds.clamp(
            self.camera
                .to_world(self.thumbnail.clamp(point), self.thumbnail),
        )
    }
}

impl Canvas {
    pub(super) fn paint_navigator(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        viewport: Rect,
        navigator: &Navigator,
    ) {
        let painter = ui.painter_at(navigator.panel.intersect(ui.clip_rect()));
        painter.rect_filled(navigator.panel, 6.0, self.style.background);
        painter.rect_stroke(
            navigator.panel,
            6.0,
            ui.visuals().widgets.noninteractive.bg_stroke,
            StrokeKind::Inside,
        );
        let painter = painter.with_clip_rect(navigator.thumbnail.intersect(painter.clip_rect()));
        let camera = navigator.camera;
        let thumbnail = navigator.thumbnail;
        if ui.is_enabled() && painter.opacity() == 1.0 {
            self.navigator_cache
                .paint(&painter, graph, navigator, self.show_loops);
        } else {
            paint_shapes(&painter, graph_shapes(graph, navigator, self.show_loops));
        }
        if let Some(marker) = viewport_marker(
            camera,
            thumbnail,
            self.camera.rect_to_world(viewport, viewport),
        ) {
            painter.rect_filled(marker, 2.0, self.style.highlight.gamma_multiply(0.12));
            painter.rect_stroke(
                marker,
                2.0,
                Stroke::new(1.4, self.style.highlight),
                StrokeKind::Inside,
            );
        }
    }
}

fn edge_shapes(
    curve: Curve<'_>,
    color: Color32,
    camera: Camera,
    thumbnail: Rect,
) -> impl Iterator<Item = Shape> + '_ {
    let screen = move |point| camera.to_screen(point, thumbnail);
    let stroke = Stroke::new(1.0, color);
    // egui аппроксимирует кривые в масштабе мини-карты; фиксированные 24 отрезка на ребро не нужны.
    let (shape, points): (Option<Shape>, &[Pos2]) = match curve {
        Curve::Quadratic(points) => (
            Some(Shape::QuadraticBezier(
                QuadraticBezierShape::from_points_stroke(
                    points.map(screen),
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                ),
            )),
            &[],
        ),
        Curve::Cubic(points) => (
            Some(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                points.map(screen),
                false,
                Color32::TRANSPARENT,
                stroke,
            ))),
            &[],
        ),
        Curve::Polyline(points) => (None, points),
    };
    shape.into_iter().chain(
        points
            .windows(2)
            .map(move |pair| Shape::line_segment([screen(pair[0]), screen(pair[1])], stroke)),
    )
}

fn graph_shapes<'a>(
    graph: &'a (impl Graph + ?Sized),
    navigator: &'a Navigator,
    loops: bool,
) -> impl Iterator<Item = Shape> + 'a {
    let camera = navigator.camera;
    let thumbnail = navigator.thumbnail;
    let groups = (0..graph.group_count()).map(move |index| {
        let group = graph.group(index);
        let bounds = group.frame.bounds();
        Shape::rect_stroke(
            Rect::from_min_max(
                camera.to_screen(bounds.min, thumbnail),
                camera.to_screen(bounds.max, thumbnail),
            ),
            0.0,
            Stroke::new(1.0, group.color),
            StrokeKind::Inside,
        )
    });
    let edges = (0..graph.edge_count())
        .map(move |index| graph.edge(index))
        .filter(move |edge| loops || !edge.is_loop)
        .flat_map(move |edge| {
            edge_shapes(edge.path.unrounded_curve(), edge.color, camera, thumbnail)
        });
    let nodes = (0..graph.node_count()).map(move |index| {
        let node = graph.node(index);
        Shape::circle_filled(camera.to_screen(node.position, thumbnail), 2.0, node.color)
    });
    groups.chain(edges).chain(nodes)
}

fn paint_shapes(painter: &Painter, mut shapes: impl Iterator<Item = Shape>) {
    // Методы пользовательского Graph вызываются до захвата блокировки egui.
    let mut batch = smallvec::SmallVec::<[Shape; 64]>::new();
    loop {
        batch.extend(shapes.by_ref().take(64));
        if batch.is_empty() {
            break;
        }
        painter.extend(batch.drain(..));
    }
}

fn viewport_marker(camera: Camera, thumbnail: Rect, visible: Rect) -> Option<Rect> {
    let marker = Rect::from_min_max(
        camera.to_screen(visible.min, thumbnail),
        camera.to_screen(visible.max, thumbnail),
    )
    .intersect(thumbnail);
    if !marker.is_positive() {
        return None;
    }
    let size = marker
        .size()
        .max(Vec2::splat(MIN_MARKER_SIZE))
        .min(thumbnail.size());
    let min = (marker.center() - size * 0.5).clamp(thumbnail.min, thumbnail.max - size);
    Some(Rect::from_min_size(min, size))
}

#[cfg(test)]
mod tests;
