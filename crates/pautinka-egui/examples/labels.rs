mod support;

use egui::{Color32, Pos2, Rect};
use pautinka_egui::{
    Canvas, Edge, Graph, GraphId, GraphRevision, Node, NodeId, Renderer,
    labels::{LabelLayoutOptions, LabelPlacement, LabelPosition, LabelPositions, NodeLabelOptions},
};

struct Stations {
    id: GraphId,
    positions: [Pos2; 60],
    names: [String; 60],
}

impl Default for Stations {
    fn default() -> Self {
        Self {
            id: GraphId::fresh(),
            positions: std::array::from_fn(|i| {
                egui::pos2((i % 10) as f32 * 45.0, (i / 10) as f32 * 32.0)
            }),
            names: std::array::from_fn(|i| format!("Station {i}: North district")),
        }
    }
}

impl Graph for Stations {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }
    fn node_count(&self) -> usize {
        self.positions.len()
    }
    fn edge_count(&self) -> usize {
        0
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(
            NodeId::new(index as u64),
            self.positions[index],
            &self.names[index],
        )
    }
    fn edge(&self, _: usize) -> Edge<'_> {
        unreachable!("Scene has no edges")
    }
    fn bounds(&self) -> Rect {
        Rect::from_points(&self.positions)
    }
}

#[derive(Default)]
struct LabelsRenderer {
    vertical: bool,
}

impl Renderer for LabelsRenderer {
    fn node_label_options(&self, index: usize, _: Node<'_>) -> NodeLabelOptions {
        NodeLabelOptions {
            priority: if index.is_multiple_of(10) { 100 } else { 0 },
            placement: LabelPlacement::Auto {
                allowed: if self.vertical {
                    LabelPositions::new(LabelPosition::North).with(LabelPosition::South)
                } else {
                    LabelPositions::ALL
                },
                preferred: LabelPosition::North,
            },
            ..NodeLabelOptions::default()
        }
    }
}

struct App {
    graph: Stations,
    canvas: Canvas,
    renderer: LabelsRenderer,
    options: LabelLayoutOptions,
    enabled: bool,
}

impl Default for App {
    fn default() -> Self {
        let mut canvas = Canvas::default();
        canvas.style.label_min_zoom = 0.0;
        Self {
            graph: Stations::default(),
            canvas,
            renderer: LabelsRenderer::default(),
            options: LabelLayoutOptions::default(),
            enabled: true,
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::Panel::left("settings").show(ui, |ui| {
            ui.heading("Smart labels");
            ui.checkbox(&mut self.enabled, "Enable automatic placement");
            ui.checkbox(&mut self.renderer.vertical, "Only above / below");
            ui.add(egui::Slider::new(&mut self.options.gap, 0.0..=20.0).text("Node gap"));
            ui.add(
                egui::Slider::new(&mut self.options.label_padding, 0.0..=12.0).text("Label gap"),
            );
            let mut budget = self.options.max_labels.min(60);
            ui.add(egui::Slider::new(&mut budget, 0..=60).text("Label budget"));
            self.options.max_labels = budget;
            ui.add(
                egui::Slider::new(&mut self.options.switch_margin, 0.0..=1.0)
                    .text("Position stability"),
            );
            ui.checkbox(&mut self.options.emphasis.hovered, "Always show hovered");
            ui.checkbox(&mut self.options.emphasis.selected, "Always show selected");
            ui.label("Every tenth station has a higher priority. Pan and zoom to explore.");
            ui.colored_label(
                Color32::LIGHT_BLUE,
                format!("Visible labels: {}", self.canvas.node_labels().len()),
            );
        });
        egui::CentralPanel::default().show(ui, |ui| {
            self.canvas.options.labels = self.enabled.then_some(self.options);
            self.canvas.show_with(ui, &self.graph, &self.renderer);
        });
    }
}

fn main() -> eframe::Result {
    support::run("Smart node labels", App::default())
}
