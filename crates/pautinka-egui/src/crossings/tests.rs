use std::{cell::Cell, collections::BTreeMap};

use egui::{Color32, Pos2, pos2};

use crate::{Direction, Edge, EdgeId, Node, NodeId, geometry::EdgePath};

use super::*;

mod bridges;
mod cache;
mod curves;
mod gaps;

struct TestGraph {
    id: GraphId,
    revision: GraphRevision,
    nodes: usize,
    paths: Vec<(EdgeId, EdgePath)>,
    changes: pautinka_core::GeometryChanges,
    journal_reads: Cell<usize>,
    direction: Direction,
}

impl TestGraph {
    fn crossing() -> Self {
        Self::new([
            [pos2(0.0, -100.0), pos2(0.0, 100.0)],
            [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        ])
    }

    fn new(lines: impl IntoIterator<Item = [Pos2; 2]>) -> Self {
        Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default(),
            nodes: 0,
            changes: pautinka_core::GeometryChanges::default(),
            journal_reads: Cell::new(0),
            direction: Direction::Undirected,
            paths: lines
                .into_iter()
                .enumerate()
                .map(|(index, [a, b])| {
                    (EdgeId::new(index as u64), EdgePath::try_line(a, b).unwrap())
                })
                .collect(),
        }
    }
}

impl Graph for TestGraph {
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(crate::Selection),
    ) -> crate::ChangeCoverage {
        self.journal_reads.set(self.journal_reads.get() + 1);
        self.changes.visit(since, self.revision, visit)
    }
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn node_count(&self) -> usize {
        self.nodes
    }
    fn edge_count(&self) -> usize {
        self.paths.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(NodeId::new(index as u64), Pos2::ZERO, "")
    }
    fn edge(&self, index: usize) -> crate::Edge<'_> {
        let (id, path) = &self.paths[index];
        Edge {
            id: *id,
            path,
            color: Color32::WHITE,
            direction: self.direction,
            is_loop: false,
        }
    }
    fn bounds(&self) -> Rect {
        let initial = if self.nodes == 0 {
            Rect::NOTHING
        } else {
            Rect::ZERO
        };
        self.paths
            .iter()
            .fold(initial, |bounds, (_, path)| bounds.union(path.bounds()))
    }
}

fn prepare(graph: &TestGraph) -> CrossingCache {
    let mut spatial = SpatialCache::default();
    spatial.synchronize(graph).unwrap();
    let mut cache = CrossingCache::default();
    cache.prepare(graph, &spatial);
    cache
}

fn pieces(
    cache: &CrossingCache,
    graph: &TestGraph,
    edge: usize,
    zoom: f32,
    gap: f32,
) -> Vec<Vec<Pos2>> {
    let mut result = Vec::new();
    let overrides = BTreeMap::new();
    let view = CrossingView {
        graph,
        defaults: CrossingOptions {
            size: CrossingSize::Fixed(
                CrossingPoints::new(gap).unwrap_or_else(|| CrossingPoints::new(0.0).unwrap()),
            ),
            ..CrossingOptions::default()
        },
        overrides: &overrides,
        zoom,
        width: &|_| 1.6,
    };
    cache.pieces(edge, &view, |points| result.push(points.to_vec()));
    result
}

fn decorated(
    cache: &CrossingCache,
    graph: &TestGraph,
    index: usize,
    zoom: f32,
    defaults: CrossingOptions,
    overrides: &BTreeMap<crate::EdgeId, EdgeCrossings>,
) -> Vec<(bool, Vec<Pos2>)> {
    let view = CrossingView {
        graph,
        defaults,
        overrides,
        zoom,
        width: &|i| if i == 0 { 4.0 } else { 8.0 },
    };
    let mut parts = Vec::new();
    cache.pieces(index, &view, |points| parts.push((false, points.to_vec())));
    cache.bridges(index, None, &view, |points| {
        parts.push((true, points.to_vec()));
    });
    parts
}
