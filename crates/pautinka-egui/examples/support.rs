#![allow(dead_code)]

use egui::{Color32, Pos2, Rect, Vec2, pos2};
use pautinka_core::GeometryChanges;
use pautinka_egui::{
    Canvas, ChangeCoverage, Direction, Edge, EdgeId, ElementId, Graph, GraphId, GraphRevision,
    Node, NodeId, NodeInteraction, NodeShape, Port, PortId, RoutingMode, Selection,
    geometry::EdgePath,
};

pub struct Network {
    id: GraphId,
    positions: [Pos2; 3],
    ports: [[Port; 2]; 3],
    paths: [EdgePath; 2],
    bounds: Rect,
    revision: GraphRevision,
    changes: GeometryChanges,
}

impl Default for Network {
    fn default() -> Self {
        let positions = [pos2(-180.0, 0.0), pos2(0.0, 0.0), pos2(180.0, 80.0)];
        let ports = positions.map(ports);
        let paths = std::array::from_fn(|i| route(&ports, i));
        let mut result = Self {
            id: GraphId::fresh(),
            positions,
            ports,
            paths,
            bounds: Rect::ZERO,
            revision: GraphRevision::default(),
            changes: GeometryChanges::default(),
        };
        result.update_bounds();
        result
    }
}

fn shape() -> NodeShape {
    NodeShape::rectangle(Vec2::new(80.0, 40.0)).unwrap()
}
fn ports(position: Pos2) -> [Port; 2] {
    [-Vec2::X, Vec2::X].map(|direction| Port {
        id: PortId::new(u64::from(direction.x > 0.0)),
        anchor: shape().anchor(position, direction).unwrap(),
    })
}
fn route(ports: &[[Port; 2]; 3], index: usize) -> EdgePath {
    RoutingMode::Curved
        .route_anchors(
            ports[index][1].anchor,
            ports[index + 1][0].anchor,
            40.0,
            0.0,
        )
        .unwrap()
}

impl Network {
    pub fn position(&self, id: NodeId) -> Option<Pos2> {
        Self::index(id).map(|index| self.positions[index])
    }
    fn index(id: NodeId) -> Option<usize> {
        [10, 20, 30].iter().position(|&value| value == id.get())
    }
    pub fn move_node(&mut self, id: NodeId, position: Pos2) {
        let Some(index) = Self::index(id) else {
            return;
        };
        if self.positions[index] == position {
            return;
        }
        self.positions[index] = position;
        self.ports[index] = ports(position);
        let mut changed = [Selection::Node(index); 3];
        let mut count = 1;
        for edge in 0..2 {
            if edge == index || edge + 1 == index {
                self.paths[edge] = route(&self.ports, edge);
                changed[count] = Selection::Edge(edge);
                count += 1;
            }
        }
        let before = self.revision;
        self.revision = before.next_geometry();
        self.changes
            .record(before, self.revision, changed.into_iter().take(count));
        self.update_bounds();
    }
    fn update_bounds(&mut self) {
        self.bounds = self
            .paths
            .iter()
            .fold(Rect::NOTHING, |bounds, path| bounds.union(path.bounds()));
        for &position in &self.positions {
            self.bounds = self.bounds.union(shape().bounds(position));
        }
    }
}

impl Graph for Network {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn node_count(&self) -> usize {
        3
    }
    fn edge_count(&self) -> usize {
        2
    }
    fn node(&self, index: usize) -> Node<'_> {
        let mut node = Node::new(
            NodeId::new([10, 20, 30][index]),
            self.positions[index],
            ["Pump", "Valve", "Tank"][index],
        );
        node.color = [
            Color32::LIGHT_BLUE,
            Color32::from_rgb(230, 90, 60),
            Color32::LIGHT_GREEN,
        ][index];
        node
    }
    fn node_shape(&self, _: usize) -> Option<NodeShape> {
        Some(shape())
    }
    fn node_ports(&self, index: usize) -> &[Port] {
        &self.ports[index]
    }
    fn node_interaction(&self, index: usize) -> NodeInteraction {
        NodeInteraction {
            selectable: true,
            draggable: index != 2,
        }
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        let mut edge = Edge::new(EdgeId::new(100 + index as u64), &self.paths[index]);
        edge.color = Color32::from_rgb(80, 170, 230);
        edge.direction = Direction::Forward;
        edge
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        self.changes.visit(since, self.revision, visit)
    }
    fn resolve(&self, id: ElementId) -> Option<Selection> {
        match id {
            pautinka_egui::ElementId::Group(_) => None,
            ElementId::Node(id) => Self::index(id).map(Selection::Node),
            ElementId::Edge(id) => id
                .get()
                .checked_sub(100)
                .filter(|&index| index < 2)
                .map(|index| Selection::Edge(index as usize)),
        }
    }
}

pub fn run(title: &str, app: impl eframe::App + 'static) -> eframe::Result {
    eframe::run_native(
        title,
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1000.0, 650.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_| Ok(Box::new(app))),
    )
}

pub fn toolbar(ui: &mut egui::Ui, canvas: &mut Canvas, graph: &impl Graph, title: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.strong(title);
        ui.separator();
        if ui.button("Fit graph").on_hover_text("Home").clicked() {
            canvas.camera.fit = true;
        }
        if ui
            .add_enabled(
                canvas.selected().is_some(),
                egui::Button::new("Focus selection"),
            )
            .on_hover_text("F")
            .clicked()
            && let Some(selection) = canvas.selected()
        {
            canvas.focus(selection, graph);
        }
        ui.menu_button("Controls", |ui| {
            egui::Grid::new("controls").show(ui, |ui| {
                for (gesture, action) in [
                    ("Drag background", "Pan"),
                    ("Scroll / pinch", "Zoom at pointer"),
                    ("Double-click background / Home", "Fit graph"),
                    ("Ctrl/Command + click", "Toggle selection"),
                    ("Shift + drag background", "Select a region"),
                    ("Arrow keys", "Select nearby node"),
                    ("Ctrl/Command + A", "Select all"),
                    ("F", "Focus selection"),
                    ("Escape", "Cancel gesture / clear selection"),
                ] {
                    ui.label(gesture);
                    ui.weak(action);
                    ui.end_row();
                }
            });
        });
        ui.separator();
        ui.weak(format!("{} selected", canvas.selected_elements().count()));
    });
    ui.separator();
}
