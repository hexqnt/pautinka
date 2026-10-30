use egui::{Color32, Pos2, Rect, Vec2, pos2};
use pautinka_core::GeometryChanges;

use crate::{Direction, Edge, EdgeId, Node, NodeId, NodeShape, PreparedGraph, geometry::EdgePath};

use super::*;

struct TestGraph {
    id: GraphId,
    revision: GraphRevision,
    changes: GeometryChanges,
    nodes: [Pos2; 4],
    paths: [EdgePath; 5],
    wide_node: bool,
    node_margin: Option<f32>,
    edge_margins: [f32; 2],
}

impl Default for TestGraph {
    fn default() -> Self {
        let nodes = [
            pos2(-100.0, 0.0),
            pos2(100.0, 0.0),
            pos2(0.0, 150.0),
            pos2(100.0, 0.0),
        ];
        Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default(),
            changes: GeometryChanges::default(),
            nodes,
            paths: [
                EdgePath::new(nodes[0], nodes[1], -40.0, 0),
                EdgePath::new(nodes[1], nodes[0], -40.0, 1),
                EdgePath::new(nodes[1], nodes[2], 0.0, 0),
                EdgePath::new(nodes[2], nodes[2], 0.0, 0),
                EdgePath::try_polyline([nodes[0], pos2(-100.0, 100.0), pos2(0.0, 100.0), nodes[2]])
                    .unwrap(),
            ],
            wide_node: false,
            node_margin: None,
            edge_margins: [7.0, 16.0],
        }
    }
}

impl Graph for TestGraph {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        self.changes.visit(since, self.revision, visit)
    }
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn edge_count(&self) -> usize {
        self.paths.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node {
            id: NodeId::new(index as u64),
            position: self.nodes[index],
            label: "Node",
            accessibility_label: "Node",
            radius: 6.0,
            color: Color32::WHITE,
        }
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        Edge {
            id: EdgeId::new(index as u64),
            path: &self.paths[index],
            color: Color32::WHITE,
            direction: Direction::Forward,
            is_loop: index == 3,
        }
    }
    fn node_shape(&self, index: usize) -> Option<NodeShape> {
        (self.wide_node && index == 0)
            .then(|| NodeShape::rectangle(Vec2::new(1000.0, 300.0)).unwrap())
    }
    fn node_screen_margin(&self, index: usize) -> f32 {
        self.node_margin
            .unwrap_or_else(|| 32.0 + self.node_geometry(index).screen_margin())
    }
    fn edge_pick_radius(&self, _: usize) -> f32 {
        self.edge_margins[0]
    }
    fn edge_screen_margin(&self, _: usize) -> f32 {
        self.edge_margins[1]
    }
    fn bounds(&self) -> Rect {
        self.paths
            .iter()
            .fold(Rect::NOTHING, |bounds, path| bounds.union(path.bounds()))
    }
}

#[test]
fn rectangle_selection_contains_the_rendered_shape_at_small_zoom() {
    let graph = TestGraph {
        wide_node: true,
        ..Default::default()
    };
    let mut canvas = Canvas::default();
    canvas.camera.zoom = 0.001;
    let node = graph.node(0);
    canvas.select_rect(
        graph.node_shape(0).unwrap().bounds(node.position),
        false,
        &graph,
    );
    assert!(!canvas.is_selected(crate::ElementId::Node(node.id)));

    canvas.select_rect(
        Rect::from_center_size(node.position, Vec2::splat(4.0 / canvas.camera.zoom)),
        false,
        &graph,
    );
    assert!(canvas.is_selected(crate::ElementId::Node(node.id)));
}

#[test]
fn indexed_picking_matches_linear_geometry_at_every_zoom_and_loop_setting() {
    for wide_node in [false, true] {
        let graph = TestGraph {
            wide_node,
            ..Default::default()
        };
        let mut indexed = Canvas::default();
        indexed.prepare(&graph).unwrap();
        let mut linear = Canvas::default();
        for zoom in [0.1, 0.5, 1.0, 3.0, 20.0] {
            indexed.camera.zoom = zoom;
            linear.camera.zoom = zoom;
            for loops in [true, false] {
                indexed.set_show_loops(loops, &graph);
                linear.set_show_loops(loops, &graph);
                for x in -40..41 {
                    for y in -30..51 {
                        let point = pos2(x as f32 * 5.0, y as f32 * 5.0);
                        assert_eq!(
                            indexed.pick(point, &graph),
                            linear.pick(point, &graph),
                            "point={point:?}, zoom={zoom}, loops={loops}"
                        );
                    }
                }
            }
        }
        assert_eq!(indexed.stats().rebuilds, 1);
    }
}

#[test]
fn route_part_counts_change_locally_without_rebuilding_the_scene_index() {
    let mut graph = TestGraph::default();
    let mut canvas = Canvas::default();
    canvas.prepare(&graph).unwrap();
    let initial = canvas.stats().spatial_entries;
    let old_parts = graph.paths[0].spatial_chunks().len();
    for count in [3, 20, 1] {
        let before = graph.revision;
        graph.revision = before.next_geometry();
        graph.paths[0] = EdgePath::try_polyline(
            (0..=count)
                .map(|i| pos2(-300.0 + i as f32 * 20.0, -200.0 + (i % 2) as f32 * 20.0))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        graph
            .changes
            .record(before, graph.revision, [Selection::Edge(0)]);
        canvas.prepare(&graph).unwrap();
        assert_eq!(canvas.stats().rebuilds, 1);
        assert_eq!(canvas.stats().spatial_entries, initial - old_parts + count);
        let point = graph.paths[0].points()[0].lerp(graph.paths[0].points()[1], 0.5);
        assert_eq!(canvas.pick(point, &graph), Some(Selection::Edge(0)));
    }
    assert!(canvas.stats().updated_entries > 0);
}

#[test]
fn bulk_motion_and_changing_route_parts_preserve_picking_and_visibility() {
    let mut graph = TestGraph::default();
    let mut cache = SpatialCache::default();
    cache.synchronize(&graph).unwrap();
    for (round, segments) in [1, 24, 3, 1].into_iter().enumerate() {
        let before = graph.revision;
        graph.revision = before.next_geometry();
        for node in &mut graph.nodes {
            *node += Vec2::new(30.0, -15.0);
        }
        graph.paths = [
            EdgePath::try_polyline(
                (0..=segments)
                    .map(|i| pos2(-200.0 + i as f32 * 30.0, (i % 2) as f32 * 30.0))
                    .collect::<Vec<_>>(),
            )
            .unwrap(),
            EdgePath::try_line(graph.nodes[1], graph.nodes[0]).unwrap(),
            EdgePath::new(graph.nodes[1], graph.nodes[2], 40.0, 0),
            EdgePath::new(graph.nodes[2], graph.nodes[2], 0.0, 1),
            EdgePath::new(graph.nodes[0], graph.nodes[2], -40.0, 0),
        ];
        graph.changes.record(
            before,
            graph.revision,
            (0..graph.node_count())
                .map(Selection::Node)
                .chain((0..graph.edge_count()).map(Selection::Edge)),
        );
        cache.synchronize(&graph).unwrap();
        assert_eq!(cache.stats().rebuilds, 1);
        assert_eq!(cache.stats().spatial_refits, round as u64 + 1);
        assert_eq!(
            cache.changed_edges_since(before, &graph),
            Some([0, 1, 2, 3, 4].as_slice())
        );
        let mut fresh = SpatialCache::default();
        fresh.synchronize(&graph).unwrap();
        assert_eq!(cache.bounds(), fresh.bounds());
        assert_eq!(cache.stats().spatial_entries, fresh.stats().spatial_entries);
        assert_eq!(
            cache.slot_heap_bytes,
            cache.edge_slots.iter().map(slot_heap_bytes).sum()
        );
        for zoom in [0.1, 1.0, 3.0] {
            for loops in [false, true] {
                for x in -8..16 {
                    for y in -8..12 {
                        let point = pos2(x as f32 * 25.0, y as f32 * 25.0);
                        let query = Rect::from_center_size(point, Vec2::splat(10.0));
                        cache.visible(&graph, query, zoom, loops);
                        fresh.visible(&graph, query, zoom, loops);
                        assert_eq!(cache.visible_nodes, fresh.visible_nodes);
                        assert_eq!(cache.visible_edges, fresh.visible_edges);
                        let point = crate::HitPoint::from_world(point).unwrap();
                        assert_eq!(
                            cache.pick(point, zoom, loops, &graph),
                            fresh.pick(point, zoom, loops, &graph)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn prepared_cache_checks_instance_and_revision_and_preserves_camera_and_selection() {
    let first = TestGraph::default();
    let mut second = TestGraph::default();
    let mut canvas = Canvas::default();
    assert!(!canvas.install_prepared(PreparedGraph::new(&first).unwrap(), &second));
    let stale = PreparedGraph::new(&second).unwrap();
    second.revision = second.revision.next_geometry();
    assert!(!canvas.install_prepared(stale, &second));
    canvas.camera.center = pos2(34.0, 56.0);
    canvas.camera.zoom = 1.7;
    canvas.select(Some(Selection::Node(2)), &second);
    let prepared = PreparedGraph::new(&second).unwrap();
    assert_eq!(prepared.graph_id(), second.id);
    assert_eq!(prepared.revision(), second.revision);
    assert!(canvas.install_prepared(prepared, &second));
    assert_eq!(canvas.camera.center, pos2(34.0, 56.0));
    assert_eq!(canvas.camera.zoom, 1.7);
    assert_eq!(canvas.selected(), Some(Selection::Node(2)));
}

#[test]
fn visibility_uses_shape_bounds_and_excludes_empty_parts_of_long_routes() {
    let mut graph = TestGraph {
        wide_node: true,
        ..Default::default()
    };
    graph.paths[0] = EdgePath::try_polyline([
        pos2(-100.0, -100.0),
        pos2(100.0, -100.0),
        pos2(100.0, 100.0),
        pos2(-100.0, 100.0),
    ])
    .unwrap();
    let mut cache = SpatialCache::default();
    cache.synchronize(&graph).unwrap();
    cache.visible(
        &graph,
        Rect::from_center_size(Pos2::ZERO, Vec2::splat(20.0)),
        10.0,
        true,
    );
    assert!(cache.visible_nodes.contains(&0));
    assert!(!cache.visible_edges.contains(&0));
    assert!(graph.paths[0].bounds().contains(Pos2::ZERO));
}

#[test]
fn invalid_edge_margins_in_full_and_local_updates_invalidate_the_cache() {
    for margin in 0..2 {
        for value in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut graph = TestGraph::default();
            graph.edge_margins[margin] = value;
            let mut cache = SpatialCache::default();
            assert_eq!(cache.synchronize(&graph), Err(SpatialError::InvalidBounds));
            assert!(!cache.ready(&graph));

            graph.edge_margins = [7.0, 16.0];
            cache.synchronize(&graph).unwrap();
            let before = graph.revision;
            graph.revision = before.next_geometry();
            graph.edge_margins[margin] = value;
            graph
                .changes
                .record(before, graph.revision, [Selection::Edge(0)]);
            assert_eq!(cache.synchronize(&graph), Err(SpatialError::InvalidBounds));
            assert!(!cache.ready(&graph));

            graph.edge_margins = [7.0, 16.0];
            cache.synchronize(&graph).unwrap();
            assert!(cache.ready(&graph));
            assert_eq!(cache.stats().rebuilds, 2);
        }
    }
}

#[test]
fn node_margins_keep_the_geometry_fallback_and_reject_infinity_in_both_updates() {
    let mut graph = TestGraph::default();
    let mut cache = SpatialCache::default();
    for value in [-1.0, f32::NAN, f32::NEG_INFINITY] {
        graph.node_margin = Some(value);
        assert_eq!(node_bounds(&graph, 0).unwrap().1, 38.0);
    }
    graph.node_margin = Some(f32::INFINITY);
    assert_eq!(cache.synchronize(&graph), Err(SpatialError::InvalidBounds));
    graph.node_margin = None;
    cache.synchronize(&graph).unwrap();

    let before = graph.revision;
    graph.revision = before.next_geometry();
    graph.node_margin = Some(f32::INFINITY);
    graph
        .changes
        .record(before, graph.revision, [Selection::Node(0)]);
    assert_eq!(cache.synchronize(&graph), Err(SpatialError::InvalidBounds));
    assert!(!cache.ready(&graph));
    graph.node_margin = None;
    cache.synchronize(&graph).unwrap();
    assert_eq!(cache.stats().rebuilds, 2);
}

#[test]
fn local_updates_deduplicate_changes_and_reset_stamps_on_overflow() {
    let mut graph = TestGraph::default();
    let mut cache = SpatialCache::default();
    cache.synchronize(&graph).unwrap();
    cache.stamp = u64::MAX;
    cache.node_stamps.fill(1);
    cache.edge_stamps.fill(1);
    let before = graph.revision;
    graph.revision = before.next_geometry();
    graph.nodes[0] += Vec2::X;
    graph.paths[0] = EdgePath::try_line(graph.nodes[0], graph.nodes[1]).unwrap();
    graph.changes.record(
        before,
        graph.revision,
        [
            Selection::Node(0),
            Selection::Edge(0),
            Selection::Node(0),
            Selection::Edge(0),
        ],
    );
    cache.synchronize(&graph).unwrap();
    assert!(cache.ready(&graph));
    assert_eq!(cache.stats().rebuilds, 1);
    assert_eq!(
        cache.stats().updated_entries,
        1 + graph.paths[0].spatial_chunks().len() as u64
    );
    assert_eq!(cache.node_stamps[0], 1);
    assert_eq!(cache.edge_stamps[0], 1);
}

#[test]
fn sparse_motion_uses_incremental_updates() {
    let mut graph = TestGraph::default();
    let mut cache = SpatialCache::default();
    cache.synchronize(&graph).unwrap();
    let before = graph.revision;
    graph.revision = before.next_geometry();
    graph.nodes[0] += Vec2::X;
    graph
        .changes
        .record(before, graph.revision, [Selection::Node(0)]);
    cache.synchronize(&graph).unwrap();
    assert_eq!(cache.stats().spatial_refits, 0);
    assert_eq!(cache.stats().updated_entries, 1);
    assert_eq!(cache.stats().rebuilds, 1);
}

#[test]
fn rejected_bulk_update_invalidates_the_cache_and_can_be_rebuilt() {
    let mut graph = TestGraph::default();
    let mut cache = SpatialCache::default();
    cache.synchronize(&graph).unwrap();
    let before = graph.revision;
    graph.revision = before.next_geometry();
    for node in &mut graph.nodes {
        *node += Vec2::splat(1000.0);
    }
    graph.changes.record(
        before,
        graph.revision,
        (0..graph.node_count())
            .map(Selection::Node)
            .chain((0..graph.edge_count()).map(Selection::Edge)),
    );
    graph.edge_margins[0] = f32::NAN;
    assert_eq!(cache.synchronize(&graph), Err(SpatialError::InvalidBounds));
    assert!(!cache.ready(&graph));
    graph.edge_margins[0] = 7.0;
    cache.synchronize(&graph).unwrap();
    assert!(cache.ready(&graph));
    assert_eq!(cache.stats().rebuilds, 2);
    for &node in &graph.nodes {
        let index = graph
            .nodes
            .iter()
            .rposition(|&position| position == node)
            .unwrap();
        assert_eq!(
            cache.pick(
                crate::HitPoint::from_world(node).unwrap(),
                1.0,
                true,
                &graph
            ),
            Some(Selection::Node(index))
        );
    }
}
