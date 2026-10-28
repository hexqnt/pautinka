use egui::{Align2, Stroke, StrokeKind, Vec2};

use crate::{Group, GroupState, GroupVisual, RenderContext};

/// Штатная рамка с заголовком; дополнительные символы можно дорисовать в собственном renderer.
pub fn paint_default_group(context: &RenderContext<'_>, group: Group<'_>, visual: &GroupVisual) {
    let painter = context.painter;
    let style = &visual.style;
    if let Some(halo) = style.halo {
        painter.rect_stroke(visual.rect.expand(3.0), 5.0, halo, StrokeKind::Outside);
    }
    painter.rect(
        visual.rect,
        5.0,
        style.fill,
        style.stroke,
        StrokeKind::Inside,
    );
    let header = painter.with_clip_rect(visual.header_rect.intersect(painter.clip_rect()));
    header.rect_filled(visual.header_rect, 0.0, style.header_fill);
    let mut label_rect = visual.header_rect.shrink2(Vec2::new(6.0, 0.0));
    if let Some(toggle) = visual.toggle_rect {
        let center = toggle.center();
        let radius = toggle.size().min_elem().min(12.0) * 0.3;
        let [start, tip, end] = if group.state == Some(GroupState::Collapsed) {
            [
                center + Vec2::new(-radius * 0.5, -radius),
                center + Vec2::new(radius, 0.0),
                center + Vec2::new(-radius * 0.5, radius),
            ]
        } else {
            [
                center + Vec2::new(-radius, -radius * 0.5),
                center + Vec2::new(0.0, radius),
                center + Vec2::new(radius, -radius * 0.5),
            ]
        };
        let stroke = Stroke::new(1.5, style.label_color);
        // Два отрезка сохраняют значок на стеке без отдельного Vec на каждую рамку и кадр.
        header.line_segment([start, tip], stroke);
        header.line_segment([tip, end], stroke);
        label_rect.min.x = toggle.max.x + 4.0;
    }
    if label_rect.is_positive() {
        header
            .with_clip_rect(label_rect.intersect(header.clip_rect()))
            .text(
                label_rect.left_center(),
                Align2::LEFT_CENTER,
                group.label,
                style.label_font.clone(),
                style.label_color,
            );
    }
}
