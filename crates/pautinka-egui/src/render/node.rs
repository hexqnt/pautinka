use egui::{
    Color32, Painter, Pos2, Rect, Shape, Stroke, StrokeKind,
    epaint::{CircleShape, EllipseShape, RectShape},
};

use crate::{NodeShape, NodeVisual, RenderContext, ShapeKind};

pub(super) fn paint(context: &RenderContext<'_>, visual: &NodeVisual, stroke: Stroke) {
    let painter = context.painter;
    let style = &visual.style;
    if let Some(halo) = style.halo {
        paint_shape(
            painter,
            visual.shape,
            visual.rect.expand(4.0),
            Color32::TRANSPARENT,
            halo,
            StrokeKind::Outside,
            false,
        );
    }
    let separate = !stroke.is_empty()
        && style.fill != Color32::TRANSPARENT
        && (!style.fill.is_opaque() || !stroke.color.is_opaque() || painter.opacity() != 1.0);
    paint_shape(
        painter,
        visual.shape,
        visual.rect,
        style.fill,
        stroke,
        StrokeKind::Inside,
        separate,
    );
}

fn paint_shape(
    painter: &Painter,
    node: Option<NodeShape>,
    rect: Rect,
    fill: Color32,
    stroke: Stroke,
    kind: StrokeKind,
    separate: bool,
) {
    let Some(node) = node else {
        paint_circle(painter, rect, fill, stroke, separate);
        return;
    };
    match node.kind() {
        ShapeKind::Circle => paint_circle(painter, rect, fill, stroke, separate),
        ShapeKind::Ellipse => paint_layers(
            painter,
            (rect.center(), rect.size() * 0.5),
            fill,
            stroke,
            separate,
            |(center, radius), fill, stroke| {
                let mut ellipse = EllipseShape::filled(center, radius, fill);
                ellipse.stroke = stroke;
                Shape::Ellipse(ellipse)
            },
        ),
        ShapeKind::Rectangle if node.corner_radius().get() == 0.0 => paint_layers(
            painter,
            rect,
            fill,
            stroke,
            separate,
            |rect, fill, stroke| Shape::Rect(RectShape::new(rect, 0.0, fill, stroke, kind)),
        ),
        ShapeKind::Rectangle
        | ShapeKind::Triangle
        | ShapeKind::Diamond
        | ShapeKind::RegularPolygon { .. } => {
            // egui владеет точками PathShape; это итоговый буфер отрисовки, а не промежуточная геометрия модели.
            let mut points = Vec::with_capacity(node.outline_capacity());
            let center = rect.center();
            let size = rect.size();
            node.outline(|point| points.push(center + point.to_vec2() / node.size() * size));
            paint_layers(
                painter,
                points,
                fill,
                stroke,
                separate,
                Shape::convex_polygon,
            );
        }
    }
}

fn paint_circle(painter: &Painter, rect: Rect, fill: Color32, stroke: Stroke, separate: bool) {
    paint_layers(
        painter,
        (rect.center(), rect.width() * 0.5),
        fill,
        stroke,
        separate,
        |(center, radius): (Pos2, f32), fill, stroke| {
            Shape::Circle(CircleShape {
                center,
                radius,
                fill,
                stroke,
            })
        },
    );
}

fn paint_layers<T: Clone>(
    painter: &Painter,
    geometry: T,
    mut fill: Color32,
    stroke: Stroke,
    separate: bool,
    shape: impl Fn(T, Color32, Stroke) -> Shape,
) {
    if separate {
        // Полупрозрачные слои смешиваются последовательно; копируется лишь геометрия, которой egui владеет для каждого слоя.
        painter.add(shape(geometry.clone(), fill, Stroke::NONE));
        fill = Color32::TRANSPARENT;
    }
    painter.add(shape(geometry, fill, stroke));
}
