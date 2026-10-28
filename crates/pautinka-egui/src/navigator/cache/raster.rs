use egui::{Color32, ColorImage, Pos2, Rect};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

use crate::{Graph, geometry::Curve};

use super::super::Navigator;

pub(super) fn image(
    graph: &(impl Graph + ?Sized),
    navigator: &Navigator,
    loops: bool,
    pixels_per_point: f32,
    anti_alias: bool,
) -> Option<ColorImage> {
    let size = navigator.thumbnail.size();
    let width = u32::try_from((size.x * pixels_per_point).ceil() as i64).ok()?;
    let height = u32::try_from((size.y * pixels_per_point).ceil() as i64).ok()?;
    let mut pixmap = Pixmap::new(width, height)?;
    // Локальные координаты позволяют перемещать панель без пересоздания текстуры.
    let thumbnail = Rect::from_min_size(Pos2::ZERO, size);
    let screen = |point| navigator.camera.to_screen(point, thumbnail);
    let transform = Transform::from_scale(width as f32 / size.x, height as f32 / size.y);
    let mut paint = Paint {
        anti_alias,
        ..Paint::default()
    };
    let stroke = Stroke {
        width: 1.0,
        ..Stroke::default()
    };
    for index in 0..graph.group_count() {
        let group = graph.group(index);
        let bounds = group.frame.bounds();
        let min = screen(bounds.min);
        let max = screen(bounds.max);
        if let Some(rect) =
            tiny_skia::Rect::from_ltrb(min.x + 0.5, min.y + 0.5, max.x - 0.5, max.y - 0.5)
        {
            set_color(&mut paint, group.color);
            pixmap.stroke_path(
                &PathBuilder::from_rect(rect),
                &paint,
                &stroke,
                transform,
                None,
            );
        }
    }
    for index in 0..graph.edge_count() {
        let edge = graph.edge(index);
        if !loops && edge.is_loop {
            continue;
        }
        if let Some(path) = edge_path(edge.path.unrounded_curve(), screen) {
            set_color(&mut paint, edge.color);
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        }
    }
    for index in 0..graph.node_count() {
        let node = graph.node(index);
        let center = screen(node.position);
        if let Some(path) = PathBuilder::from_circle(center.x, center.y, 2.0) {
            set_color(&mut paint, node.color);
            pixmap.fill_path(&path, &paint, FillRule::Winding, transform, None);
        }
    }
    Some(ColorImage::from_rgba_premultiplied(
        [width as usize, height as usize],
        pixmap.data(),
    ))
}

fn set_color(paint: &mut Paint<'_>, color: Color32) {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    paint.set_color_rgba8(r, g, b, a);
}

fn edge_path(curve: Curve<'_>, screen: impl Fn(Pos2) -> Pos2) -> Option<Path> {
    let mut path = PathBuilder::new();
    match curve {
        Curve::Polyline(points) => {
            let (&first, rest) = points.split_first()?;
            let first = screen(first);
            path.move_to(first.x, first.y);
            for &point in rest {
                let point = screen(point);
                path.line_to(point.x, point.y);
            }
        }
        Curve::Quadratic(points) => {
            let [a, b, c] = points.map(screen);
            path.move_to(a.x, a.y);
            path.quad_to(b.x, b.y, c.x, c.y);
        }
        Curve::Cubic(points) => {
            let [a, b, c, d] = points.map(screen);
            path.move_to(a.x, a.y);
            path.cubic_to(b.x, b.y, c.x, c.y, d.x, d.y);
        }
    }
    path.finish()
}

#[cfg(test)]
mod tests;
