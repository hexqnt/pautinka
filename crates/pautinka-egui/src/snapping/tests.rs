use egui::{Pos2, Rect, pos2};

use crate::{Edge, GeometryChanges, Node, NodeInteraction, SnapOptions};

use super::*;

struct Fixture {
    id: GraphId,
    revision: GraphRevision,
    points: Vec<Pos2>,
    changes: GeometryChanges,
    journal: bool,
    pinned: Option<usize>,
}

impl Fixture {
    fn move_node(&mut self, index: usize, position: Pos2) {
        let before = self.revision;
        self.points[index] = position;
        self.revision = before.next_geometry();
        self.changes
            .record(before, self.revision, [Selection::Node(index)]);
    }
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default(),
            points: vec![pos2(-100.0, 0.0), pos2(100.0, 0.0)],
            changes: GeometryChanges::default(),
            journal: true,
            pinned: None,
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
        self.points.len()
    }
    fn edge_count(&self) -> usize {
        0
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(NodeId::new(index as u64), self.points[index], "")
    }
    fn node_interaction(&self, index: usize) -> NodeInteraction {
        NodeInteraction {
            draggable: self.pinned != Some(index),
            ..NodeInteraction::default()
        }
    }
    fn edge(&self, _: usize) -> Edge<'_> {
        unreachable!("fixture has no edges")
    }
    fn bounds(&self) -> Rect {
        Rect::from_points(&self.points)
    }
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        if self.journal {
            self.changes.visit(since, self.revision, visit)
        } else {
            ChangeCoverage::Unavailable
        }
    }
}

#[test]
fn moving_nodes_reuse_the_index_but_sources_and_unknown_changes_rebuild_it() {
    let mut graph = Fixture::default();
    let mut cache = SnapCache::default();
    let selection = SelectionState::default();
    let dragged = NodeId::new(0);
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    for x in 0..100 {
        graph.move_node(0, pos2(x as f32, 0.0));
        cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    }
    assert_eq!(cache.rebuilds, 1);
    graph.move_node(1, pos2(50.0, 10.0));
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    assert_eq!(cache.rebuilds, 2);
    assert_eq!(
        cache
            .index
            .snap(pos2(52.0, 12.0), SnapOptions::default(), None)
            .position,
        pos2(50.0, 10.0)
    );
    graph.journal = false;
    graph.move_node(0, Pos2::ZERO);
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    assert_eq!(cache.rebuilds, 3);
}

#[test]
fn graph_topology_selection_and_appearance_invalidate_cached_sources() {
    let mut graph = Fixture::default();
    let mut cache = SnapCache::default();
    let mut selection = SelectionState::default();
    let dragged = NodeId::new(0);
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    graph.revision = graph.revision.next_appearance();
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    assert_eq!(cache.rebuilds, 2);
    selection
        .members
        .extend([ElementId::Node(dragged), ElementId::Node(NodeId::new(1))]);
    selection.epoch += 1;
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    assert_eq!(cache.rebuilds, 3);
    assert_eq!(
        cache
            .index
            .snap(pos2(101.0, 1.0), SnapOptions::default(), None)
            .target,
        None
    );
    graph.points.pop();
    graph.revision = graph.revision.next_topology();
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    assert_eq!(cache.rebuilds, 4);
    graph.id = GraphId::fresh();
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    assert_eq!(cache.rebuilds, 5);
}

#[test]
fn changing_group_pins_updates_sources_with_and_without_geometry_changes() {
    let mut graph = Fixture::default();
    let mut cache = SnapCache::default();
    let mut selection = SelectionState::default();
    let dragged = NodeId::new(0);
    selection
        .members
        .extend([ElementId::Node(dragged), ElementId::Node(NodeId::new(1))]);
    cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
    let query = pos2(102.0, 2.0);
    let policy = SnapOptions::default();
    assert_eq!(cache.index.snap(query, policy, None).target, None);

    for (pinned, geometry_changed) in [(true, true), (false, true), (true, false)] {
        graph.pinned = pinned.then_some(1);
        if geometry_changed {
            graph.move_node(1, graph.points[1]);
        } else {
            graph.revision = graph.revision.next_appearance();
        }
        cache.synchronize(&graph, dragged, &selection, SnapKinds::AXES);
        let result = cache.index.snap(query, policy, None);
        assert_eq!(result.guides().count(), if pinned { 2 } else { 0 });
        assert!(result.guides().all(|guide| guide.source == graph.points[1]));
    }
    assert_eq!(cache.rebuilds, 4);
}
