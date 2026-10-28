mod canvas;
mod curves;
mod geometry;
mod options;
mod render;

#[cfg(test)]
use egui::Rect;

use crate::{Graph, GraphId, GraphRevision, geometry::EdgePath, spatial::SpatialCache};

#[cfg(test)]
use render::bridge_points;

use geometry::section_clearance;
#[cfg(test)]
use geometry::{Intersection, intersection};

pub use options::*;
pub(crate) use render::CrossingView;

#[derive(Clone, Copy, Debug)]
struct Crossing {
    edge: usize,
    other: usize,
    segment: usize,
    fraction: f32,
    sine: f32,
    horizontal: bool,
    other_horizontal: bool,
    other_segment: usize,
    other_fraction: f32,
    clearance: f32,
    distance: f32,
}

#[derive(Default)]
pub(super) struct CrossingCache {
    key: Option<(GraphId, GraphRevision)>,
    crossings: Vec<Crossing>,
    // Диапазоны по плотному индексу ребра: usize × (N + 1); ёмкость переиспользуется при перестройке.
    offsets: Vec<usize>,
    dirty: Vec<usize>,
    pub full_rebuilds: u64,
    pub scanned_edges: u64,
}

impl CrossingCache {
    pub fn prepare(&mut self, graph: &(impl Graph + ?Sized), spatial: &SpatialCache) {
        if self.ready(graph) {
            return;
        }
        self.dirty.clear();
        let local = self.key.is_some_and(|(id, previous)| {
            if id != graph.id() || previous.topology() != graph.revision().topology() {
                return false;
            }
            if let Some(edges) = spatial.changed_edges_since(previous, graph) {
                self.dirty.extend_from_slice(edges);
                return true;
            }
            let mut valid = true;
            let coverage = graph.geometry_changes(previous, &mut |element| {
                if let crate::Selection::Edge(index) = element {
                    if index < graph.edge_count() {
                        self.dirty.push(index);
                    } else {
                        valid = false;
                    }
                }
            });
            self.dirty.sort_unstable();
            self.dirty.dedup();
            valid && coverage == crate::ChangeCoverage::Complete
        });
        // При движении всего графа нечего сохранять; полный проход также исключает поиск в dirty для каждой пары.
        if local && self.dirty.len() < graph.edge_count() {
            self.crossings.retain(|crossing| {
                self.dirty.binary_search(&crossing.edge).is_err()
                    && self.dirty.binary_search(&crossing.other).is_err()
            });
            for position in 0..self.dirty.len() {
                self.scan(graph, spatial, self.dirty[position], true);
            }
        } else {
            if !local {
                self.full_rebuilds += 1;
            }
            self.crossings.clear();
            let eligible = (0..graph.edge_count())
                .filter(|&index| !graph.edge(index).is_loop)
                .count();
            // В разреженном графе общий BVH содержит слишком много посторонних пар узлов и групп.
            let unrelated =
                graph.node_count() + graph.group_count() + graph.edge_count() - eligible;
            if eligible < 2 {
                self.scanned_edges += eligible as u64;
            } else if eligible < unrelated {
                for index in 0..graph.edge_count() {
                    self.scan(graph, spatial, index, false);
                }
            } else {
                self.scanned_edges += eligible as u64;
                spatial.edge_segment_pairs(|first, second| self.scan_pair(graph, first, second));
            }
        }
        self.crossings.sort_unstable_by(|a, b| {
            (a.edge, a.segment)
                .cmp(&(b.edge, b.segment))
                .then(a.fraction.total_cmp(&b.fraction))
                .then(a.other.cmp(&b.other))
        });
        self.crossings.dedup_by(|a, b| {
            a.edge == b.edge
                && a.other == b.other
                && a.segment == b.segment
                && a.fraction == b.fraction
        });
        self.offsets.clear();
        if !self.crossings.is_empty() {
            self.offsets.resize(graph.edge_count() + 1, 0);
            for crossing in &self.crossings {
                self.offsets[crossing.edge + 1] += 1;
            }
            let mut total = 0;
            for offset in &mut self.offsets {
                total += *offset;
                *offset = total;
            }
        }
        self.key = Some((graph.id(), graph.revision()));
    }

    fn scan(
        &mut self,
        graph: &(impl Graph + ?Sized),
        spatial: &SpatialCache,
        index: usize,
        local: bool,
    ) {
        let edge = graph.edge(index);
        if edge.is_loop {
            return;
        }
        self.scanned_edges += 1;
        for (chunk, bounds) in edge.path.spatial_chunks().enumerate() {
            spatial.edge_segments(bounds, |other, other_chunk| {
                if index > other && (!local || self.dirty.binary_search(&other).is_ok()) {
                    return;
                }
                self.scan_pair(graph, (index, chunk), (other, other_chunk));
            });
        }
    }

    fn scan_pair(
        &mut self,
        graph: &(impl Graph + ?Sized),
        (index, chunk): (usize, usize),
        (other, other_chunk): (usize, usize),
    ) {
        if index == other {
            return;
        }
        let edge = graph.edge(index);
        let other_edge = graph.edge(other);
        if edge.is_loop || other_edge.is_loop {
            return;
        }
        let (Some(segments), Some(other_segments)) = (
            edge.path.spatial_chunk_segments(chunk),
            other_edge.path.spatial_chunk_segments(other_chunk),
        ) else {
            return;
        };
        for segment in segments {
            for other_segment in other_segments.clone() {
                self.scan_segments(
                    (index, segment),
                    edge.path,
                    (other, other_segment),
                    other_edge.path,
                );
            }
        }
    }

    fn scan_segments(
        &mut self,
        (index, segment): (usize, usize),
        path: &EdgePath,
        (other_index, other_segment): (usize, usize),
        other_path: &EdgePath,
    ) {
        let Some(curves::PathIntersection {
            first,
            second,
            sine,
        }) = curves::refined_intersection(path, segment, other_path, other_segment)
        else {
            return;
        };
        let clearance = section_clearance(path, first.segment, first.fraction).min(
            section_clearance(other_path, second.segment, second.fraction),
        );
        let horizontal = curves::horizontal(path, first);
        let other_horizontal = curves::horizontal(other_path, second);
        self.crossings.extend([
            Crossing {
                edge: index,
                other: other_index,
                other_segment: second.segment,
                other_fraction: second.fraction,
                segment: first.segment,
                fraction: first.fraction,
                sine,
                clearance,
                horizontal,
                other_horizontal,
                distance: first.distance(path),
            },
            Crossing {
                edge: other_index,
                other: index,
                other_segment: first.segment,
                other_fraction: first.fraction,
                segment: second.segment,
                fraction: second.fraction,
                sine,
                clearance,
                horizontal: other_horizontal,
                other_horizontal: horizontal,
                distance: second.distance(other_path),
            },
        ]);
    }

    pub(super) fn ready(&self, graph: &(impl Graph + ?Sized)) -> bool {
        self.key.is_some_and(|(id, revision)| {
            id == graph.id() && revision.same_geometry(graph.revision())
        })
    }
}

#[cfg(test)]
mod tests;
