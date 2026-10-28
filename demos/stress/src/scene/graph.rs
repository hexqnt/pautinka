use egui::Rect;
use pautinka::egui::{
    ChangeCoverage, Edge, EdgeId, ElementId, Graph, GraphId, GraphRevision, Node, NodeId,
    NodeShape, Selection,
};

use super::{Load, Scene};

impl Graph for Scene {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn edge_count(&self) -> usize {
        self.edges.len()
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
    fn node(&self, index: usize) -> Node<'_> {
        let node = &self.nodes[index];
        Node {
            id: NodeId::new(index as u64),
            position: node.position,
            label: &node.label,
            accessibility_label: &node.label,
            radius: self.style.appearance.radius(),
            color: self.style.appearance.color(index),
        }
    }
    fn node_shape(&self, index: usize) -> Option<NodeShape> {
        Some(self.shape(index))
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        let edge = &self.edges[index];
        Edge {
            id: EdgeId::new(
                edge.ends[0] as u64 * u64::from(Load::MAX_EDGES_PER_NODE) + u64::from(edge.lane),
            ),
            path: &edge.path,
            color: self.edge_colors[edge.ends[0] % usize::from(self.style.appearance.colors())],
            direction: self.style.direction,
            is_loop: false,
        }
    }
    fn resolve(&self, id: ElementId) -> Option<Selection> {
        match id {
            ElementId::Node(id) => self.node_index(id).map(Selection::Node),
            ElementId::Edge(id) => {
                let source =
                    usize::try_from(id.get() / u64::from(Load::MAX_EDGES_PER_NODE)).ok()?;
                let lane = (id.get() % u64::from(Load::MAX_EDGES_PER_NODE)) as u8;
                if source >= self.nodes.len() {
                    return None;
                }
                let range = self.load.edge_range(source);
                (usize::from(lane) < range.len())
                    .then_some(Selection::Edge(range.start + usize::from(lane)))
            }
            ElementId::Group(_) => None,
        }
    }
}
