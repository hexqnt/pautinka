use egui::{Pos2, Rect, pos2};

use crate::{Edge, EdgeId, EdgePath, GraphId, GraphRevision, Node, NodeId};

use super::*;

struct Fixture {
    id: GraphId,
    revision: GraphRevision,
    nodes: Vec<NodeId>,
    edge: EdgePath,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default(),
            nodes: [7, 20, 30].map(NodeId::new).into(),
            edge: EdgePath::new(Pos2::ZERO, pos2(100.0, 0.0), 0.0, 0),
        }
    }
}

impl Graph for Fixture {
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
        1
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(self.nodes[index], pos2(index as f32 * 50.0, 0.0), "")
    }
    fn edge(&self, _: usize) -> Edge<'_> {
        Edge::new(EdgeId::new(7), &self.edge)
    }
    fn bounds(&self) -> Rect {
        self.edge.bounds()
    }
}

#[test]
fn replacing_selection_is_atomic_deduplicated_and_preserves_typed_ids() {
    let graph = Fixture::default();
    let mut canvas = Canvas::default();
    assert!(canvas.focus(Selection::Node(1), &graph));
    let revision = canvas.selection_revision();
    assert!(!canvas.select_elements(
        [
            ElementId::Node(NodeId::new(7)),
            ElementId::Edge(EdgeId::new(20))
        ],
        &graph,
    ));
    assert_eq!(canvas.selected(), Some(Selection::Node(1)));
    assert_eq!(canvas.selected_elements().count(), 1);
    assert_eq!(canvas.pending_focus, Some(Selection::Node(1)));
    assert_eq!(canvas.selection_revision(), revision);

    let elements = [
        ElementId::Node(NodeId::new(7)),
        ElementId::Edge(EdgeId::new(7)),
        ElementId::Node(NodeId::new(7)),
    ];
    assert!(canvas.select_elements(elements, &graph));
    assert_eq!(canvas.selected_elements().count(), 2);
    assert!(elements.into_iter().all(|id| canvas.is_selected(id)));
    assert_eq!(canvas.pending_focus, None);
    assert_eq!(canvas.selection_revision(), revision + 1);
    assert!(canvas.select_elements(elements, &graph));
    assert_eq!(canvas.selection_revision(), revision + 1);
}

#[test]
fn synchronization_remaps_primary_and_removes_only_missing_ids() {
    let mut graph = Fixture::default();
    let mut canvas = Canvas::default();
    canvas.select_all(&graph);
    assert!(canvas.set_primary(ElementId::Node(NodeId::new(30)), &graph));
    canvas.synchronize(&graph);
    let revision = canvas.selection_revision();

    graph.nodes.reverse();
    graph.revision = graph.revision.next_topology();
    canvas.synchronize(&graph);
    assert_eq!(canvas.selected(), Some(Selection::Node(0)));
    assert_eq!(canvas.selected_id(), Some(ElementId::Node(NodeId::new(30))));
    assert_eq!(canvas.selected_elements().count(), 4);
    assert_eq!(canvas.selection_revision(), revision);

    graph.nodes.remove(1);
    graph.revision = graph.revision.next_topology();
    canvas.synchronize(&graph);
    assert_eq!(canvas.selected_id(), Some(ElementId::Node(NodeId::new(30))));
    assert!(!canvas.is_selected(ElementId::Node(NodeId::new(20))));
    assert_eq!(canvas.selection_revision(), revision + 1);

    graph.nodes.remove(0);
    graph.revision = graph.revision.next_topology();
    canvas.synchronize(&graph);
    assert_eq!(canvas.selected(), Some(Selection::Node(0)));
    assert_eq!(canvas.selected_id(), Some(ElementId::Node(NodeId::new(7))));
    assert!(canvas.is_selected(ElementId::Edge(EdgeId::new(7))));
    assert_eq!(canvas.selection_revision(), revision + 2);
}

#[test]
fn reset_clears_selection_and_invalid_focus_preserves_it() {
    let graph = Fixture::default();
    let mut canvas = Canvas::default();
    assert!(canvas.focus(Selection::Node(1), &graph));
    assert!(!canvas.focus(Selection::Node(3), &graph));
    assert!(!canvas.focus(Selection::Edge(1), &graph));
    assert_eq!(canvas.selected(), Some(Selection::Node(1)));
    canvas.reset();
    assert_eq!(canvas.selected(), None);
    assert!(canvas.camera.fit);
}
