use egui::{Color32, Pos2, Rect, Ui, Vec2, pos2};
use pautinka_egui::{
    Canvas, Edge, Graph, GraphRevision, Node, RouteEndpoints, RoutingMode, Selection,
    geometry::EdgePath,
};

struct DemoGraph {
    id: pautinka_egui::GraphId,
    nodes: [Pos2; 3],
    edges: [EdgePath; 4],
    bounds: Rect,
    routing: RoutingMode,
    revision: GraphRevision,
}

impl DemoGraph {
    fn set_routing(&mut self, mode: RoutingMode) {
        if self.routing == mode {
            return;
        }
        self.routing = mode;
        for (index, (a, b, offset)) in [(0, 1, -40.0), (1, 0, 40.0), (1, 2, 0.0), (2, 2, 0.0)]
            .into_iter()
            .enumerate()
        {
            let endpoints = if a == b {
                RouteEndpoints::Loop {
                    position: self.nodes[a],
                }
            } else {
                RouteEndpoints::Between {
                    from: self.nodes[a],
                    to: self.nodes[b],
                }
            };
            let offset = if mode == RoutingMode::Curved && a > b {
                -offset
            } else {
                offset
            };
            self.edges[index] = mode.route(endpoints, offset, 0).unwrap();
        }
        self.bounds = path_bounds(&self.edges);
        self.revision = self.revision.next_geometry();
    }
}

impl Default for DemoGraph {
    fn default() -> Self {
        let nodes = [pos2(-150.0, 0.0), pos2(150.0, 0.0), pos2(0.0, 180.0)];
        let edges = [
            EdgePath::new(nodes[0], nodes[1], -40.0, 0),
            EdgePath::new(nodes[1], nodes[0], -40.0, 1),
            EdgePath::new(nodes[1], nodes[2], 0.0, 0),
            EdgePath::new(nodes[2], nodes[2], 0.0, 0),
        ];
        let bounds = path_bounds(&edges);
        Self {
            id: pautinka_egui::GraphId::fresh(),
            nodes,
            edges,
            bounds,
            routing: RoutingMode::default(),
            revision: GraphRevision::default(),
        }
    }
}

impl Graph for DemoGraph {
    fn id(&self) -> pautinka_egui::GraphId {
        self.id
    }
    fn revision(&self) -> pautinka_egui::GraphRevision {
        self.revision
    }

    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
    }

    fn node(&self, index: usize) -> Node<'_> {
        let label = ["Alpha", "Beta", "Gamma"][index];
        Node {
            id: pautinka_egui::NodeId::new(index as u64),
            position: self.nodes[index],
            label,
            accessibility_label: label,
            radius: 6.5,
            color: Color32::LIGHT_BLUE,
        }
    }

    fn edge(&self, index: usize) -> Edge<'_> {
        Edge {
            id: pautinka_egui::EdgeId::new(index as u64),
            path: &self.edges[index],
            color: Color32::LIGHT_GREEN,
            direction: pautinka_egui::Direction::Forward,
            is_loop: index == 3,
        }
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Default)]
struct Demo {
    graph: DemoGraph,
    canvas: Canvas,
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("Pautinka");
                ui.separator();
                if ui.button("Fit graph").on_hover_text("Home").clicked() {
                    self.canvas.camera.fit = true;
                }
                if ui.button("Focus Alpha").clicked() {
                    self.canvas.focus(Selection::Node(0), &self.graph);
                }
                let mut loops = self.canvas.show_loops();
                if ui.checkbox(&mut loops, "Loops").changed() {
                    self.canvas.set_show_loops(loops, &self.graph);
                }
                let mut routing = self.graph.routing;
                egui::ComboBox::from_id_salt("routing")
                    .selected_text(format!("{routing:?}"))
                    .show_ui(ui, |ui| {
                        for mode in [
                            RoutingMode::Curved,
                            RoutingMode::Straight,
                            RoutingMode::Orthogonal,
                            RoutingMode::Octilinear,
                            RoutingMode::Angular15,
                        ] {
                            ui.selectable_value(&mut routing, mode, format!("{mode:?}"));
                        }
                    });
                self.graph.set_routing(routing);
                let selection = match self.canvas.selected() {
                    Some(Selection::Node(index)) => self.graph.node(index).label,
                    Some(Selection::Edge(_)) => "Connection",
                    Some(Selection::Group(_)) => "Group",
                    None => "None",
                };
                ui.weak(format!("Selection: {selection}"));
            });
            ui.weak("Drag background to pan · Scroll to zoom · Shift + drag to select · Home to fit · F to focus");
            ui.separator();
            self.canvas.show(ui, &self.graph);
        });
    }
}

fn path_bounds(paths: &[EdgePath]) -> Rect {
    paths
        .iter()
        .fold(Rect::NOTHING, |bounds, path| bounds.union(path.bounds()))
        .expand(60.0)
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Graph canvas",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size(Vec2::new(800.0, 600.0)),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|_| Ok(Box::<Demo>::default())),
    )
}
