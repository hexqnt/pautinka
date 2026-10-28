use egui::{Color32, FontId, Stroke, Vec2};
use pautinka::egui::{
    DefaultRenderer, Edge, EdgeStyle, ElementState, LinePattern, Node, NodeVisual, RenderContext,
    Renderer, Style,
};

use crate::metro::{Line, Metro, Route, StationAccess, StationId};

pub const PAPER: Color32 = Color32::from_rgb(246, 245, 240);
pub const INK: Color32 = Color32::from_rgb(37, 51, 65);
pub const ACCENT: Color32 = Color32::from_rgb(238, 164, 91);

pub struct MetroRenderer<'a> {
    pub metro: &'a Metro,
    pub line: Option<Line>,
    pub route: Option<&'a Route>,
    pub endpoints: [Option<StationId>; 2],
}

impl MetroRenderer<'_> {
    fn node_active(&self, index: usize) -> bool {
        self.route.map_or_else(
            || {
                self.line
                    .is_none_or(|line| self.metro.stations[index].line == line)
            },
            |route| route.stations.contains(&StationId(index)),
        )
    }
}

impl Renderer for MetroRenderer<'_> {
    fn edge_style(
        &self,
        index: usize,
        edge: Edge<'_>,
        state: ElementState,
        style: &Style,
    ) -> EdgeStyle {
        let mut result = DefaultRenderer.edge_style(index, edge, state, style);
        let connection = &self.metro.connections[index];
        let active = self.route.map_or_else(
            || {
                self.line.is_none_or(|line| {
                    self.metro.stations[connection.from.0].line == line
                        || self.metro.stations[connection.to.0].line == line
                })
            },
            |route| route.edges.contains(&index),
        );
        result.stroke = Stroke::new(
            if active { 4.0 } else { 2.0 },
            if active {
                edge.color
            } else {
                Color32::from_rgb(219, 222, 221)
            },
        );
        result.halo = None;
        if self.metro.is_transfer(index) {
            result.stroke.width = 2.0;
            result.pattern = LinePattern::Dashed {
                dash: 3.0,
                gap: 3.0,
            };
        }
        result
    }

    fn paint_node(&self, context: &RenderContext<'_>, node: Node<'_>, visual: &NodeVisual) {
        let station = &self.metro.stations[visual.index];
        let active = self.node_active(visual.index);
        let center = visual.rect.center();
        let selected = visual.state.emphasized();
        let endpoint = self.endpoints.contains(&Some(StationId(visual.index)));
        let color = if active {
            node.color
        } else {
            Color32::from_rgb(200, 207, 207)
        };
        if selected || endpoint {
            context
                .painter
                .circle_filled(center, 11.0, color.gamma_multiply(0.18));
            context
                .painter
                .circle_stroke(center, 8.0, Stroke::new(1.5, color));
        }
        context.painter.circle(
            center,
            if endpoint { 5.5 } else { 4.0 },
            PAPER,
            Stroke::new(2.0, color),
        );
        if station.access != StationAccess::Open {
            context.painter.line_segment(
                [center - Vec2::splat(3.0), center + Vec2::splat(3.0)],
                Stroke::new(1.5, color),
            );
        }
        if visual.show_label || selected || endpoint {
            let offset = station.label_offset * context.camera.zoom;
            let offset = if offset.length() < 8.0 {
                offset.normalized() * 8.0
            } else {
                offset
            };
            let label_color = if selected || endpoint {
                INK
            } else if active {
                Color32::from_rgb(75, 87, 99)
            } else {
                Color32::from_rgb(169, 178, 181)
            };
            let galley = context.painter.layout_no_wrap(
                station.name.clone(),
                FontId::proportional((17.0 * context.camera.zoom).clamp(9.0, 24.0)),
                label_color,
            );
            let rect = station
                .label_align
                .anchor_size(center + offset, galley.size());
            context
                .painter
                .rect_filled(rect.expand(2.0), 2.0, PAPER.gamma_multiply(0.25));
            context.painter.galley(rect.min, galley, label_color);
        }
    }

    fn node_screen_margin(&self, _: usize, _: Node<'_>) -> f32 {
        500.0
    }
}

pub fn configure(context: &egui::Context) {
    egui_extras::install_image_loaders(context);
    let mut fonts = egui::FontDefinitions::default();
    // Hack содержит стрелки, которых нет в основном пропорциональном шрифте.
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("Hack".into());
    context.set_fonts(fonts);
    let mut style = (*context.global_style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::from_rgb(22, 35, 47);
    style.visuals.window_fill = Color32::from_rgb(28, 43, 56);
    style.visuals.override_text_color = Some(Color32::from_rgb(225, 231, 232));
    style.visuals.selection.bg_fill = Color32::from_rgb(72, 88, 99);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(87, 105, 114);
    style.spacing.item_spacing = Vec2::new(9.0, 9.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(14.0));
    context.set_theme(egui::Theme::Dark);
    context.set_global_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;
    use pautinka::egui::{Camera, DetailLevel, Graph};

    #[test]
    fn embedded_labels_stay_clear_of_each_other_and_station_markers() {
        let metro = Metro::embedded();
        let renderer = MetroRenderer {
            metro: &metro,
            line: None,
            route: None,
            endpoints: [None; 2],
        };
        for zoom in [0.5, 0.85, 1.0, 2.0] {
            let context = egui::Context::default();
            let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(6000.0));
            let camera = Camera {
                center: metro.bounds().center(),
                zoom,
                fit: false,
            };
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                let context = ui.ctx();
                let painter = context.layer_painter(egui::LayerId::background());
                let style = Style::default();
                let render_context = RenderContext {
                    painter: &painter,
                    camera,
                    viewport,
                    style: &style,
                    detail: DetailLevel::Full,
                };
                for index in 0..metro.node_count() {
                    let node = metro.node(index);
                    let state = ElementState::default();
                    renderer.paint_node(
                        &render_context,
                        node,
                        &NodeVisual {
                            index,
                            rect: egui::Rect::from_center_size(
                                camera.to_screen(node.position, viewport),
                                Vec2::splat(8.0),
                            ),
                            shape: None,
                            state,
                            style: renderer.node_style(index, node, state, &style),
                            show_label: true,
                        },
                    );
                }
            });
            output.textures_delta.clear();
            let mut conflicts = Vec::new();
            let labels: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape {
                        Some((text.galley.text(), text.visual_bounding_rect()))
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(labels.len(), metro.node_count());
            for (index, (name, rect)) in labels.iter().enumerate() {
                for (other, other_rect) in &labels[index + 1..] {
                    if rect.intersects(*other_rect) {
                        conflicts.push(format!("Labels overlap at zoom {zoom}: {name} / {other}"));
                    }
                }
                for station in &metro.stations {
                    let marker = egui::Rect::from_center_size(
                        camera.to_screen(station.position, viewport),
                        Vec2::splat(10.0),
                    );
                    if rect.intersects(marker) {
                        conflicts.push(format!(
                            "Label overlaps station at zoom {zoom}: {name} / {}",
                            station.name
                        ));
                    }
                }
            }
            assert!(conflicts.is_empty(), "{}", conflicts.join("\n"));
        }
    }
}
