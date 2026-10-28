mod support;

use std::collections::HashMap;

use egui::{Pos2, Rect, pos2};
use pautinka_egui::{
    Canvas, Edge, EdgeId, ElementId, Graph, GraphId, GraphRevision, Node, NodeId, Selection,
    geometry::EdgePath,
};
use petgraph::{
    stable_graph::{EdgeIndex, NodeIndex, StableDiGraph},
    visit::{EdgeRef as _, IntoEdgeReferences as _},
};

struct Equipment {
    id: NodeId,
    position: Pos2,
    name: &'static str,
}
struct Pipe {
    id: EdgeId,
    path: EdgePath,
}
struct Network {
    id: GraphId,
    graph: StableDiGraph<Equipment, Pipe>,
    nodes: Vec<NodeIndex>,
    edges: Vec<EdgeIndex>,
    indices: HashMap<ElementId, Selection>,
    bounds: Rect,
}
impl Default for Network {
    fn default() -> Self {
        let mut graph = StableDiGraph::new();
        let pump = graph.add_node(Equipment {
            id: NodeId::new(10),
            position: pos2(-100.0, 0.0),
            name: "Pump",
        });
        let removed = graph.add_node(Equipment {
            id: NodeId::new(20),
            position: Pos2::ZERO,
            name: "Removed",
        });
        let tank = graph.add_node(Equipment {
            id: NodeId::new(30),
            position: pos2(100.0, 0.0),
            name: "Tank",
        });
        graph.remove_node(removed);
        graph.add_edge(
            pump,
            tank,
            Pipe {
                id: EdgeId::new(100),
                path: EdgePath::try_line(graph[pump].position, graph[tank].position).unwrap(),
            },
        );
        // Индексы StableGraph могут иметь отверстия; плотные представления строятся при смене топологии.
        let nodes: Vec<_> = graph.node_indices().collect();
        let edges: Vec<_> = graph.edge_references().map(|edge| edge.id()).collect();
        let indices = nodes
            .iter()
            .enumerate()
            .map(|(index, &key)| (ElementId::Node(graph[key].id), Selection::Node(index)))
            .chain(
                edges
                    .iter()
                    .enumerate()
                    .map(|(index, &key)| (ElementId::Edge(graph[key].id), Selection::Edge(index))),
            )
            .collect();
        let bounds = graph.edge_references().fold(Rect::NOTHING, |bounds, edge| {
            bounds.union(edge.weight().path.bounds())
        });
        Self {
            id: GraphId::fresh(),
            graph,
            nodes,
            edges,
            indices,
            bounds,
        }
    }
}
impl Graph for Network {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        let node = &self.graph[self.nodes[index]];
        Node::new(node.id, node.position, node.name)
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        let edge = &self.graph[self.edges[index]];
        Edge::new(edge.id, &edge.path)
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn resolve(&self, id: ElementId) -> Option<Selection> {
        self.indices.get(&id).copied()
    }
}
#[derive(Default)]
struct App {
    graph: Network,
    canvas: Canvas,
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        support::toolbar(ui, &mut self.canvas, &self.graph, "petgraph adapter");
        ui.weak("Select a node or connection to explore the graph. Sparse storage uses stable application IDs.");
        self.canvas.options.interaction = pautinka_egui::InteractionOptions::read_only();
        self.canvas.show(ui, &self.graph);
    }
}
fn main() -> eframe::Result {
    support::run("petgraph adapter", App::default())
}
