use std::fmt::Write as _;

use egui::{Color32, RichText, Ui};
use pautinka::egui::Graph;

use super::Demo;
use crate::metrics::CAPACITY;

impl Demo {
    pub(super) fn refresh_statistics(&mut self) {
        let summary = self.metrics.summary;
        for line in &mut self.hud_text {
            line.clear();
        }
        let [frame, percentiles, tail, cpu, elements, samples, index] = &mut self.hud_text;
        if summary.mean_ms > 0.0 {
            write!(frame, "FPS {:>6.1}", 1_000.0 / summary.mean_ms).unwrap();
        } else {
            frame.push_str("FPS     --");
        }
        write!(frame, "   FRAME {:>6.2} ms", summary.mean_ms).unwrap();
        write!(
            percentiles,
            "p50 {:>6.2}   p95 {:>6.2} ms",
            summary.p50_ms, summary.p95_ms
        )
        .unwrap();
        if summary.count >= 100 {
            write!(tail, "p99 {:>6.2}", summary.p99_ms).unwrap();
        } else {
            tail.push_str("p99     --");
        }
        write!(tail, "   MAX {:>6.2} ms", summary.max_ms).unwrap();
        write!(
            cpu,
            "CPU model {:.2} / widget {:.2} ms",
            summary.model_ms, summary.widget_ms
        )
        .unwrap();
        write!(
            elements,
            "N {}/{}  E {}/{}",
            self.visible.0,
            self.scene.node_count(),
            self.visible.1,
            self.scene.edge_count()
        )
        .unwrap();
        write!(samples, "SAMPLES {}/{CAPACITY}", summary.count).unwrap();
        write!(
            index,
            "INDEX {:.1} MiB   BUILD {:.2} ms",
            self.canvas.stats().spatial_cache_bytes as f64 / 1_048_576.0,
            self.metrics.last_build.unwrap_or_default().as_secs_f64() * 1_000.0
        )
        .unwrap();
    }

    pub(super) fn statistics(&self, ui: &Ui) {
        if !self.viewport.is_positive() {
            return;
        }
        egui::Area::new(egui::Id::unique("stress-hud"))
            .accessible_name("Performance HUD")
            .order(egui::Order::Middle)
            .interactable(false)
            .fixed_pos(self.viewport.min + egui::Vec2::splat(12.0))
            .constrain_to(self.viewport)
            .show(ui.ctx(), |ui| {
                ui.set_max_width((self.viewport.width() - 24.0).min(320.0));
                let frame = egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(8, 13, 19, 190))
                    .corner_radius(5.0)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        for (text, color) in self.hud_text.iter().zip([
                            Color32::from_rgb(153, 245, 170),
                            Color32::LIGHT_GRAY,
                            Color32::LIGHT_GRAY,
                            Color32::from_rgb(245, 212, 143),
                            Color32::LIGHT_GRAY,
                            Color32::LIGHT_GRAY,
                            Color32::LIGHT_GRAY,
                        ]) {
                            hud_line(ui, text, color);
                        }
                    });
                let rect = frame.response.rect;
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_bounds(egui::accesskit::Rect {
                        x0: f64::from(rect.min.x),
                        y0: f64::from(rect.min.y),
                        x1: f64::from(rect.max.x),
                        y1: f64::from(rect.max.y),
                    });
                });
            });
    }
}

fn hud_line(ui: &mut Ui, text: &str, color: Color32) {
    ui.label(RichText::new(text).monospace().size(14.0).color(color));
}
