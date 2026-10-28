use egui::{Rect, Shape, Stroke, Ui, Vec2};

use crate::{Canvas, SnapTarget};

impl Canvas {
    pub(crate) fn paint_alignment(&self, ui: &Ui, viewport: Rect) {
        let Some(snap) = self.drag.as_ref().and_then(|drag| drag.snap.result) else {
            return;
        };
        let target = self.camera.to_screen(snap.position, viewport);
        if !target.is_finite() {
            return;
        }
        let painter = ui.painter_at(viewport);
        if snap.target == Some(SnapTarget::Grid) {
            painter.circle_stroke(target, 5.0, Stroke::new(1.5, self.style.highlight));
            return;
        }
        for guide in snap.guides() {
            let source = self.camera.to_screen(guide.source, viewport);
            if let Some(points) = guide_segment(source, target, guide.kind.direction(), viewport) {
                painter.extend(Shape::dashed_line(
                    &points,
                    Stroke::new(1.0, self.style.highlight),
                    5.0,
                    4.0,
                ));
            }
        }
    }
}

fn guide_segment(
    source: egui::Pos2,
    target: egui::Pos2,
    direction: Vec2,
    viewport: Rect,
) -> Option<[egui::Pos2; 2]> {
    if !source.is_finite() || !target.is_finite() {
        return None;
    }
    let parameter = (f64::from(source.x) - f64::from(target.x)) * f64::from(direction.x)
        + (f64::from(source.y) - f64::from(target.y)) * f64::from(direction.y);
    let mut start = parameter.min(0.0) - 12.0;
    let mut end = parameter.max(0.0) + 12.0;
    // Ограничение параметра линии до разбиения на штрихи сохраняет наклон и ограничивает работу viewport.
    for axis in 0..2 {
        let step = f64::from(direction[axis]);
        let origin = f64::from(target[axis]);
        if step == 0.0 {
            if origin < f64::from(viewport.min[axis]) || origin > f64::from(viewport.max[axis]) {
                return None;
            }
        } else {
            let a = (f64::from(viewport.min[axis]) - origin) / step;
            let b = (f64::from(viewport.max[axis]) - origin) / step;
            start = start.max(a.min(b));
            end = end.min(a.max(b));
        }
    }
    if start > end {
        return None;
    }
    Some([start, end].map(|parameter| {
        egui::pos2(
            (f64::from(target.x) + f64::from(direction.x) * parameter) as f32,
            (f64::from(target.y) + f64::from(direction.y) * parameter) as f32,
        )
    }))
}
