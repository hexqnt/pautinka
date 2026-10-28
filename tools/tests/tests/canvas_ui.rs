#[path = "support/interaction.rs"]
mod support;

#[path = "canvas_ui/camera.rs"]
mod camera;
#[path = "canvas_ui/edges.rs"]
mod edges;
#[path = "canvas_ui/grid.rs"]
mod grid;
#[path = "canvas_ui/grid_snapping.rs"]
mod grid_snapping;
#[path = "canvas_ui/groups.rs"]
mod groups;
#[path = "canvas_ui/interaction.rs"]
mod interaction;
#[path = "canvas_ui/labels.rs"]
mod labels;
#[path = "canvas_ui/navigator.rs"]
mod navigator;
#[path = "canvas_ui/rendering.rs"]
mod rendering;
#[path = "canvas_ui/selection.rs"]
mod selection;
#[path = "canvas_ui/shapes.rs"]
mod shapes;
#[path = "canvas_ui/snapping.rs"]
mod snapping;

use egui::{Color32, Pos2, Rect, Ui, Vec2, pos2};
use egui_kittest::{
    Harness,
    kittest::{NodeT, Queryable},
};
use pautinka_egui::{Canvas, Edge, Graph, Node, Selection, geometry::EdgePath};

use support::{click_label, click_role_label, pointer_button};

struct TestGraph {
    id: pautinka_egui::GraphId,
    nodes: Vec<Pos2>,
    node_ids: Vec<pautinka_egui::NodeId>,
    node_color: Color32,
    node_shape: Option<pautinka_egui::NodeShape>,
    edges: [EdgePath; 2],
    revision: pautinka_egui::GraphRevision,
    second_is_loop: bool,
    first_pinned: bool,
    first_selectable: bool,
    resolve_calls: std::cell::Cell<usize>,
    bounds_padding: f32,
    direction: pautinka_egui::Direction,
}

impl Default for TestGraph {
    fn default() -> Self {
        let nodes = [pos2(-100.0, 0.0), pos2(100.0, 0.0)];
        Self {
            id: pautinka_egui::GraphId::fresh(),
            node_color: Color32::LIGHT_BLUE,
            node_shape: None,
            second_is_loop: true,
            first_pinned: false,
            first_selectable: true,
            resolve_calls: std::cell::Cell::new(0),
            bounds_padding: 60.0,
            direction: pautinka_egui::Direction::Forward,
            nodes: nodes.into(),
            node_ids: (0..2).map(pautinka_egui::NodeId::new).collect(),
            revision: pautinka_egui::GraphRevision::default(),
            edges: [
                EdgePath::new(nodes[0], nodes[1], 0.0, 0),
                EdgePath::new(nodes[1], nodes[1], 0.0, 0),
            ],
        }
    }
}

impl Graph for TestGraph {
    fn node_interaction(&self, index: usize) -> pautinka_egui::NodeInteraction {
        pautinka_egui::NodeInteraction {
            selectable: index != 0 || self.first_selectable,
            draggable: index != 0 || !self.first_pinned,
        }
    }
    fn resolve(&self, id: pautinka_egui::ElementId) -> Option<Selection> {
        self.resolve_calls.set(self.resolve_calls.get() + 1);
        match id {
            pautinka_egui::ElementId::Group(_) => None,
            pautinka_egui::ElementId::Node(id) => self
                .node_ids
                .iter()
                .position(|&candidate| candidate == id)
                .map(Selection::Node),
            pautinka_egui::ElementId::Edge(id) => (0..self.edge_count())
                .find(|&index| self.edge(index).id == id)
                .map(Selection::Edge),
        }
    }
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
        let label = ["First", "Second", "Third"][self.node_ids[index].get() as usize];
        Node {
            id: self.node_ids[index],
            position: self.nodes[index],
            label,
            accessibility_label: label,
            radius: 6.0,
            color: self.node_color,
        }
    }

    fn node_shape(&self, _: usize) -> Option<pautinka_egui::NodeShape> {
        self.node_shape
    }

    fn edge(&self, index: usize) -> Edge<'_> {
        Edge {
            id: pautinka_egui::EdgeId::new(index as u64),
            path: &self.edges[index],
            color: Color32::LIGHT_GREEN,
            direction: self.direction,
            is_loop: index == 1 && self.second_is_loop,
        }
    }

    fn bounds(&self) -> Rect {
        self.edges
            .iter()
            .fold(Rect::NOTHING, |bounds, edge| bounds.union(edge.bounds()))
            .expand(self.bounds_padding)
    }
}

#[derive(Default)]
struct State {
    graph: TestGraph,
    canvas: Canvas,
    events_seen: Vec<pautinka_egui::CanvasEvent>,
    error: Option<pautinka_egui::CanvasError>,
}

impl State {
    fn show(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut loops = self.canvas.show_loops();
            if ui.checkbox(&mut loops, "Loops").changed() {
                self.canvas.set_show_loops(loops, &self.graph);
            }
            if ui.button("Focus loop").clicked() {
                self.canvas.focus(Selection::Edge(1), &self.graph);
            }
        });
        self.error = self.canvas.try_show(ui, &self.graph).err();
        self.events_seen.extend_from_slice(self.canvas.events());
    }
}

fn canvas_harness(state: State) -> Harness<'static, State> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(|ui, state| State::show(state, ui), state)
}

fn pointer_gesture<T>(
    harness: &mut Harness<'_, T>,
    from: Pos2,
    to: Pos2,
    button: egui::PointerButton,
    modifiers: egui::Modifiers,
) {
    harness.hover_at(from);
    harness.event_modifiers(pointer_button(from, button, true, modifiers), modifiers);
    if from != to {
        harness.step();
        harness.event_modifiers(egui::Event::PointerMoved(to), modifiers);
        harness.step();
    }
    harness.event_modifiers(pointer_button(to, button, false, modifiers), modifiers);
    harness.run();
}
