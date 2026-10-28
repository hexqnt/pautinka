mod text;

use std::fmt::Write as _;

use egui::{
    Color32, FontDefinitions, Pos2, Rect, Shape, Stroke, StrokeKind,
    epaint::{ClippedShape, ColorMode, PathStroke},
};

use super::ExportError;
use text::Fonts;

pub(super) fn document(
    shapes: &[ClippedShape],
    bounds: Rect,
    background: Option<Color32>,
    fonts: &FontDefinitions,
) -> Result<String, ExportError> {
    let fonts = Fonts::new(fonts)?;
    let mut out = String::new();
    write!(out, "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">", bounds.width(), bounds.height(), bounds.min.x, bounds.min.y, bounds.width(), bounds.height()).unwrap();
    if let Some(color) = background {
        write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            bounds.min.x,
            bounds.min.y,
            bounds.width(),
            bounds.height()
        )
        .unwrap();
        paint(&mut out, color, Stroke::NONE);
        out.push_str("/>");
    }
    let mut active_clip = None;
    for (i, clipped) in shapes.iter().enumerate() {
        let clip = clipped.clip_rect.intersect(bounds);
        if !clip.is_positive() {
            continue;
        }
        if active_clip != Some(clip) {
            if active_clip.is_some() {
                out.push_str("</g>");
            }
            write!(out, "<defs><clipPath id=\"c{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#c{i})\">", clip.min.x, clip.min.y, clip.width(), clip.height()).unwrap();
            active_clip = Some(clip);
        }
        shape(&mut out, &clipped.shape, &fonts)?;
    }
    if active_clip.is_some() {
        out.push_str("</g>");
    }
    out.push_str("</svg>");
    Ok(out)
}

fn color(out: &mut String, attribute: &str, color: Color32) {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    write!(
        out,
        " {attribute}=\"#{r:02x}{g:02x}{b:02x}\" {attribute}-opacity=\"{}\"",
        f32::from(a) / 255.0
    )
    .unwrap();
}

fn paint(out: &mut String, fill: Color32, stroke: Stroke) {
    color(out, "fill", fill);
    color(out, "stroke", stroke.color);
    write!(
        out,
        " stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"",
        stroke.width.max(0.0)
    )
    .unwrap();
}

fn stroke(stroke: &PathStroke) -> Result<Stroke, ExportError> {
    let ColorMode::Solid(color) = stroke.color else {
        return Err(ExportError::UnsupportedShape("gradient strokes"));
    };
    if stroke.kind != StrokeKind::Middle {
        return Err(ExportError::UnsupportedShape("non-centered path strokes"));
    }
    Ok(Stroke::new(stroke.width, color))
}

fn points(out: &mut String, points: &[Pos2], close: bool) {
    for (i, p) in points.iter().enumerate() {
        write!(out, "{}{} {} ", if i == 0 { 'M' } else { 'L' }, p.x, p.y).unwrap();
    }
    if close {
        out.push('Z');
    }
}

fn path(out: &mut String, data: impl FnOnce(&mut String), fill: Color32, stroke: Stroke) {
    out.push_str("<path d=\"");
    data(out);
    out.push('"');
    paint(out, fill, stroke);
    out.push_str("/>");
}

fn shape(out: &mut String, shape: &Shape, fonts: &Fonts<'_>) -> Result<(), ExportError> {
    if !matches!(shape, Shape::Noop | Shape::Vec(_)) && !shape.visual_bounding_rect().is_finite() {
        return Err(ExportError::InvalidGeometry);
    }
    match shape {
        Shape::Noop => {}
        Shape::Vec(shapes) => {
            for item in shapes {
                self::shape(out, item, fonts)?;
            }
        }
        Shape::Circle(s) => {
            write!(
                out,
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"",
                s.center.x, s.center.y, s.radius
            )
            .unwrap();
            paint(out, s.fill, s.stroke);
            out.push_str("/>");
        }
        Shape::Ellipse(s) => {
            write!(
                out,
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" transform=\"rotate({} {} {})\"",
                s.center.x,
                s.center.y,
                s.radius.x,
                s.radius.y,
                s.angle.to_degrees(),
                s.center.x,
                s.center.y
            )
            .unwrap();
            paint(out, s.fill, s.stroke);
            out.push_str("/>");
        }
        Shape::LineSegment { points: p, stroke } => path(
            out,
            |out| points(out, p, false),
            Color32::TRANSPARENT,
            *stroke,
        ),
        Shape::Path(s) => path(
            out,
            |out| points(out, &s.points, s.closed),
            s.fill,
            stroke(&s.stroke)?,
        ),
        Shape::QuadraticBezier(s) => path(
            out,
            |out| {
                let [a, b, c] = s.points;
                write!(
                    out,
                    "M{} {} Q{} {} {} {}{}",
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    c.x,
                    c.y,
                    if s.closed { " Z" } else { "" }
                )
                .unwrap();
            },
            s.fill,
            stroke(&s.stroke)?,
        ),
        Shape::CubicBezier(s) => path(
            out,
            |out| {
                let [a, b, c, d] = s.points;
                write!(
                    out,
                    "M{} {} C{} {} {} {} {} {}{}",
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    c.x,
                    c.y,
                    d.x,
                    d.y,
                    if s.closed { " Z" } else { "" }
                )
                .unwrap();
            },
            s.fill,
            stroke(&s.stroke)?,
        ),
        Shape::Rect(s) => {
            if s.brush.is_some() || s.blur_width != 0.0 {
                return Err(ExportError::UnsupportedShape(
                    "textured or blurred rectangles",
                ));
            }
            let center = s.rect.center();
            write!(
                out,
                "<g transform=\"rotate({} {} {})\">",
                s.angle.to_degrees(),
                center.x,
                center.y
            )
            .unwrap();
            let inset = match s.stroke_kind {
                StrokeKind::Inside => s.stroke.width * 0.5,
                StrokeKind::Middle => 0.0,
                StrokeKind::Outside => -s.stroke.width * 0.5,
            };
            let rect = s.rect.shrink(inset);
            if rect.is_positive() {
                let radii = s.corner_radius;
                let limit = rect.width().min(rect.height()) * 0.5;
                let [nw, ne, se, sw] = [radii.nw, radii.ne, radii.se, radii.sw]
                    .map(|r| (f32::from(r) - inset).clamp(0.0, limit));
                path(
                    out,
                    |out| rounded_rect(out, rect, [nw, ne, se, sw]),
                    s.fill,
                    s.stroke,
                );
            }
            out.push_str("</g>");
        }
        Shape::Text(s) => text::paint(out, s, fonts)?,
        Shape::Mesh(_) => {
            return Err(ExportError::UnsupportedShape(
                "triangle meshes and textures",
            ));
        }
        Shape::Callback(_) => return Err(ExportError::UnsupportedShape("GPU callbacks")),
        Shape::Band(_) => return Err(ExportError::UnsupportedShape("variable-width bands")),
    }
    Ok(())
}

fn rounded_rect(out: &mut String, rect: Rect, [nw, ne, se, sw]: [f32; 4]) {
    let (left, top, right, bottom) = (rect.left(), rect.top(), rect.right(), rect.bottom());
    write!(out, "M{} {top} H{}", left + nw, right - ne).unwrap();
    write!(
        out,
        " A{ne} {ne} 0 0 1 {right} {} V{}",
        top + ne,
        bottom - se
    )
    .unwrap();
    write!(
        out,
        " A{se} {se} 0 0 1 {} {bottom} H{}",
        right - se,
        left + sw
    )
    .unwrap();
    write!(
        out,
        " A{sw} {sw} 0 0 1 {left} {} V{}",
        bottom - sw,
        top + nw
    )
    .unwrap();
    write!(out, " A{nw} {nw} 0 0 1 {} {top} Z", left + nw).unwrap();
}
