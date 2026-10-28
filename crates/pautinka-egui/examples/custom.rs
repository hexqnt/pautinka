mod support;

use egui::{Align2, Shape, Stroke, Vec2};
use pautinka_egui::{
    Canvas, DefaultRenderer, Edge, EdgeStyle, ElementState, LinePattern, Node, NodeGeometry,
    NodeVisual, RenderContext, Renderer, Style,
};

struct EquipmentRenderer;
impl Renderer for EquipmentRenderer {
    fn node_hit_test(
        &self,
        index: usize,
        node: Node<'_>,
        _: NodeGeometry,
        point: pautinka_egui::HitPoint,
        _: f32,
    ) -> Option<bool> {
        (index == 1).then(|| {
            let d = point.relative_to(node.position);
            d.x.abs() / 40.0 + d.y.abs() / 20.0 <= 1.0
        })
    }
    fn paint_node(&self, context: &RenderContext<'_>, node: Node<'_>, visual: &NodeVisual) {
        if visual.index != 1 {
            DefaultRenderer.paint_node(context, node, visual);
            return;
        }
        let r = visual.rect;
        let points = vec![
            r.center_top(),
            r.right_center(),
            r.center_bottom(),
            r.left_center(),
        ];
        if let Some(halo) = visual.style.halo {
            context.painter.add(Shape::closed_line(
                points.clone(),
                Stroke::new(halo.width + 4.0, halo.color),
            ));
        }
        context.painter.add(Shape::convex_polygon(
            points,
            visual.style.fill,
            visual.style.stroke,
        ));
        if visual.show_label {
            context.painter.text(
                r.center_bottom() + Vec2::new(0.0, 8.0),
                Align2::CENTER_TOP,
                node.label,
                visual.style.label_font.clone(),
                visual.style.label_color,
            );
        }
    }
    fn edge_style(
        &self,
        index: usize,
        edge: Edge<'_>,
        state: ElementState,
        style: &Style,
    ) -> EdgeStyle {
        let mut result = DefaultRenderer.edge_style(index, edge, state, style);
        result.stroke.width = 3.0;
        result.pattern = if index == 0 {
            LinePattern::Dotted { gap: 5.0 }
        } else {
            LinePattern::DashDot {
                dash: 8.0,
                gap: 5.0,
            }
        };
        result
    }
    fn edge_label(&self, index: usize) -> Option<&str> {
        Some(["5 bar", "Planned connection"][index])
    }
    fn edge_screen_margin(&self, _: usize, _: Edge<'_>) -> f32 {
        140.0
    }
}

#[derive(Default)]
struct App {
    graph: support::Network,
    canvas: Canvas,
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        support::toolbar(ui, &mut self.canvas, &self.graph, "Equipment network");
        ui.weak("Custom equipment, labels, dotted and dash-dot pipes. Tank is pinned.");
        self.canvas.options.interaction = pautinka_egui::InteractionOptions::read_only();
        let result = self.canvas.show_with(ui, &self.graph, &EquipmentRenderer);
        result.context_menu(|ui| {
            ui.label(format!(
                "Target: {:?}; world: {:?}",
                result.context_target, result.context_position
            ));
        });
    }
}
fn main() -> eframe::Result {
    support::run("Custom technological network", App::default())
}
