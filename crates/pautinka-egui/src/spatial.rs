mod prepared;

use egui::{Rect, Vec2};
use pautinka_core::{SpatialError, SpatialIndex, SpatialUpdateMode};
use smallvec::SmallVec;

use crate::{Canvas, ChangeCoverage, Graph, GraphId, GraphRevision, Selection};

pub use prepared::PreparedGraph;

#[derive(Clone, Copy)]
enum Indexed {
    Node(u32),
    Group(u32),
    Edge { edge: u32, chunk: u32 },
}

#[derive(Clone, Copy, PartialEq)]
struct VisibleKey {
    bounds: Rect,
    revision: GraphRevision,
    loops: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasStats {
    pub snap_rebuilds: u64,
    pub spatial_entries: usize,
    pub spatial_bytes: usize,
    pub spatial_cache_bytes: usize,
    pub node_candidates: usize,
    pub group_candidates: usize,
    pub edge_candidates: usize,
    pub rebuilds: u64,
    pub updated_entries: u64,
    /// Число завершённых массовых обновлений с общим пересчётом границ BVH.
    pub spatial_refits: u64,
    pub crossing_rebuilds: u64,
    pub crossing_scanned_edges: u64,
}

#[derive(Default)]
pub(super) struct SpatialCache {
    key: Option<(GraphId, GraphRevision)>,
    index: SpatialIndex<Indexed>,
    edge_slots: Vec<SmallVec<[u32; 8]>>,
    dirty_nodes: Vec<usize>,
    dirty_groups: Vec<usize>,
    dirty_edges: Vec<usize>,
    updated_from: Option<GraphRevision>,
    edge_stamps: Vec<u64>,
    node_stamps: Vec<u64>,
    group_stamps: Vec<u64>,
    stamp: u64,
    max_screen_margin: f32,
    visible_key: Option<VisibleKey>,
    pub visible_nodes: Vec<usize>,
    pub visible_groups: Vec<usize>,
    pub visible_edges: Vec<usize>,
    rebuilds: u64,
    updated_entries: u64,
    spatial_refits: u64,
    slot_heap_bytes: usize,
}

impl SpatialCache {
    pub fn bounds(&self) -> Rect {
        self.index.total_bounds().unwrap_or(Rect::NOTHING)
    }
    pub fn invalidate(&mut self) {
        self.key = None;
        self.updated_from = None;
        self.visible_key = None;
    }
    /// Локальное обновление уже собрало полный отсортированный набор; пропущенные версии требуют журнала.
    pub(super) fn changed_edges_since(
        &self,
        since: GraphRevision,
        graph: &(impl Graph + ?Sized),
    ) -> Option<&[usize]> {
        (self.ready(graph)
            && self
                .updated_from
                .is_some_and(|from| from.same_geometry(since)))
        .then_some(&self.dirty_edges)
    }
    pub(super) fn edge_segments(&self, bounds: Rect, mut visit: impl FnMut(usize, usize)) {
        self.index.query(bounds, |entry| {
            if let Indexed::Edge { edge, chunk } = entry {
                visit(edge as usize, chunk as usize);
            }
        });
    }

    pub(super) fn edge_segment_pairs(&self, mut visit: impl FnMut((usize, usize), (usize, usize))) {
        self.index.query_pairs(|first, second| {
            if let (
                Indexed::Edge { edge, chunk },
                Indexed::Edge {
                    edge: other,
                    chunk: other_chunk,
                },
            ) = (first, second)
            {
                visit(
                    (edge as usize, chunk as usize),
                    (other as usize, other_chunk as usize),
                );
            }
        });
    }

    pub(super) fn bridge_candidates(
        &self,
        point: crate::HitPoint,
        zoom: f32,
        height: f32,
        visit: impl FnMut(usize, usize),
    ) {
        let radius = (self.max_screen_margin + height) / zoom + point.rounding_error();
        self.edge_segments(
            Rect::from_center_size(point.rounded(), Vec2::splat(radius * 2.0)),
            visit,
        );
    }

    pub fn stats(&self) -> CanvasStats {
        CanvasStats {
            spatial_entries: self.index.len(),
            spatial_bytes: self.index.storage_bytes(),
            spatial_cache_bytes: self.index.storage_bytes()
                + self.edge_slots.capacity() * size_of::<SmallVec<[u32; 8]>>()
                + self.slot_heap_bytes
                + (self.dirty_nodes.capacity()
                    + self.dirty_groups.capacity()
                    + self.visible_groups.capacity()
                    + self.dirty_edges.capacity()
                    + self.visible_nodes.capacity()
                    + self.visible_edges.capacity())
                    * size_of::<usize>()
                + (self.edge_stamps.capacity()
                    + self.node_stamps.capacity()
                    + self.group_stamps.capacity())
                    * size_of::<u64>(),
            node_candidates: self.visible_nodes.len(),
            group_candidates: self.visible_groups.len(),
            edge_candidates: self.visible_edges.len(),
            rebuilds: self.rebuilds,
            updated_entries: self.updated_entries,
            spatial_refits: self.spatial_refits,
            crossing_rebuilds: 0,
            crossing_scanned_edges: 0,
            snap_rebuilds: 0,
        }
    }

    pub fn ready(&self, graph: &(impl Graph + ?Sized)) -> bool {
        self.key == Some((graph.id(), graph.revision()))
            && self.group_stamps.len() == graph.group_count()
            && self.node_stamps.len() == graph.node_count()
            && self.edge_slots.len() == graph.edge_count()
    }

    pub fn synchronize(&mut self, graph: &(impl Graph + ?Sized)) -> Result<(), SpatialError> {
        if self.ready(graph) {
            return Ok(());
        }
        let result = self.update(graph);
        if result.is_err() {
            self.invalidate();
        }
        result
    }

    fn update(&mut self, graph: &(impl Graph + ?Sized)) -> Result<(), SpatialError> {
        let current = graph.revision();
        let Some((id, previous)) = self.key else {
            return self.rebuild(graph);
        };
        if id != graph.id()
            || previous.topology() != current.topology()
            || self.group_stamps.len() != graph.group_count()
            || self.node_stamps.len() != graph.node_count()
            || self.edge_slots.len() != graph.edge_count()
        {
            return self.rebuild(graph);
        }
        if previous.same_geometry(current) {
            self.key = Some((id, current));
            return Ok(());
        }
        self.dirty_groups.clear();
        self.dirty_nodes.clear();
        self.dirty_edges.clear();
        self.advance_stamp();
        let mut invalid = false;
        let coverage = graph.geometry_changes(previous, &mut |element| {
            let (marks, dirty, index) = match element {
                Selection::Group(index) => (&mut self.group_stamps, &mut self.dirty_groups, index),
                Selection::Node(index) => (&mut self.node_stamps, &mut self.dirty_nodes, index),
                Selection::Edge(index) => (&mut self.edge_stamps, &mut self.dirty_edges, index),
            };
            if let Some(mark) = marks.get_mut(index) {
                if *mark != self.stamp {
                    *mark = self.stamp;
                    dirty.push(index);
                }
            } else {
                invalid = true;
            }
        });
        if coverage == ChangeCoverage::Unavailable || invalid {
            return self.rebuild(graph);
        }
        self.dirty_nodes.sort_unstable();
        self.dirty_edges.sort_unstable();
        let dirty_entries = self.dirty_nodes.len()
            + self.dirty_groups.len()
            + self
                .dirty_edges
                .iter()
                .map(|&edge| self.edge_slots[edge].len())
                .sum::<usize>();
        // От четверти записей линейный обход дешевле повторного подъёма по общим предкам.
        let mode = if dirty_entries > 0 && dirty_entries >= self.index.len().div_ceil(4) {
            SpatialUpdateMode::Refit
        } else {
            SpatialUpdateMode::Incremental
        };
        let mut updates = self.index.updates(mode);
        self.dirty_groups.sort_unstable();
        for &index in &self.dirty_groups {
            self.max_screen_margin = self.max_screen_margin.max(group_margin(graph, index)?);
            updates.update(
                updates
                    .handle(graph.node_count() + index)
                    .expect("Indexed group exists"),
                graph.group(index).frame.bounds(),
            )?;
            self.updated_entries += 1;
        }
        for &index in &self.dirty_nodes {
            let (bounds, margin) = node_bounds(graph, index)?;
            self.max_screen_margin = self.max_screen_margin.max(margin);
            updates.update(updates.handle(index).expect("Indexed node exists"), bounds)?;
            self.updated_entries += 1;
        }
        for &edge in &self.dirty_edges {
            self.max_screen_margin = self.max_screen_margin.max(edge_margin(graph, edge)?);
            let path = graph.edge(edge).path;
            let slots = &mut self.edge_slots[edge];
            let previous_heap = slot_heap_bytes(slots);
            let count = path.spatial_chunks().len();
            for &slot in slots.get(count..).unwrap_or_default() {
                updates.remove(
                    updates
                        .handle(slot as usize)
                        .expect("Indexed route part exists"),
                )?;
                self.updated_entries += 1;
            }
            slots.truncate(count);
            for (chunk, bounds) in path.spatial_chunks().enumerate() {
                if let Some(&slot) = slots.get(chunk) {
                    updates.update(
                        updates
                            .handle(slot as usize)
                            .expect("Indexed route part exists"),
                        bounds,
                    )?;
                } else {
                    slots.push(
                        updates
                            .insert(
                                Indexed::Edge {
                                    edge: edge as u32,
                                    chunk: chunk as u32,
                                },
                                bounds,
                            )?
                            .slot(),
                    );
                }
                self.updated_entries += 1;
            }
            self.slot_heap_bytes = self.slot_heap_bytes - previous_heap + slot_heap_bytes(slots);
        }
        drop(updates);
        if mode == SpatialUpdateMode::Refit {
            self.spatial_refits += 1;
        }
        self.key = Some((id, current));
        self.updated_from = Some(previous);
        self.visible_key = None;
        Ok(())
    }

    fn rebuild(&mut self, graph: &(impl Graph + ?Sized)) -> Result<(), SpatialError> {
        let mut count = graph
            .node_count()
            .checked_add(graph.group_count())
            .ok_or(SpatialError::TooManyEntries)?;
        for index in 0..graph.edge_count() {
            count = count
                .checked_add(graph.edge(index).path.spatial_chunks().len())
                .ok_or(SpatialError::TooManyEntries)?;
        }
        if count > u32::MAX as usize / 2 {
            return Err(SpatialError::TooManyEntries);
        }
        let mut entries = Vec::with_capacity(count);
        self.max_screen_margin = 32.0;
        for index in 0..graph.node_count() {
            let (bounds, margin) = node_bounds(graph, index)?;
            self.max_screen_margin = self.max_screen_margin.max(margin);
            entries.push((Indexed::Node(index as u32), bounds));
        }
        for index in 0..graph.group_count() {
            self.max_screen_margin = self.max_screen_margin.max(group_margin(graph, index)?);
            entries.push((
                Indexed::Group(index as u32),
                graph.group(index).frame.bounds(),
            ));
        }
        self.edge_slots
            .resize_with(graph.edge_count(), SmallVec::new);
        for (index, slots) in self.edge_slots.iter_mut().enumerate() {
            // Длинные маршруты переиспользуют heap-буферы и при полной перестройке.
            slots.clear();
            self.max_screen_margin = self.max_screen_margin.max(edge_margin(graph, index)?);
            for (chunk, bounds) in graph.edge(index).path.spatial_chunks().enumerate() {
                slots.push(entries.len() as u32);
                entries.push((
                    Indexed::Edge {
                        edge: index as u32,
                        chunk: chunk as u32,
                    },
                    bounds,
                ));
            }
        }
        self.index.rebuild(entries)?;
        self.slot_heap_bytes = self.edge_slots.iter().map(slot_heap_bytes).sum();
        self.edge_stamps.clear();
        self.edge_stamps.resize(graph.edge_count(), 0);
        self.group_stamps.clear();
        self.group_stamps.resize(graph.group_count(), 0);
        self.node_stamps.clear();
        self.node_stamps.resize(graph.node_count(), 0);
        self.stamp = 0;
        self.visible_groups.clear();
        self.visible_nodes.clear();
        self.visible_nodes.reserve(graph.node_count());
        self.visible_groups.reserve(graph.group_count());
        self.dirty_groups.reserve(graph.group_count());
        self.visible_edges.clear();
        self.visible_edges.reserve(graph.edge_count());
        self.dirty_groups.clear();
        self.dirty_nodes.clear();
        self.dirty_nodes.reserve(graph.node_count());
        self.dirty_edges.clear();
        self.dirty_edges.reserve(graph.edge_count());
        self.updated_from = None;
        self.key = Some((graph.id(), graph.revision()));
        self.visible_key = None;
        self.rebuilds += 1;
        Ok(())
    }

    pub fn visible(&mut self, graph: &(impl Graph + ?Sized), bounds: Rect, zoom: f32, loops: bool) {
        let bounds = bounds.expand(self.max_screen_margin / zoom);
        let key = VisibleKey {
            bounds,
            revision: graph.revision(),
            loops,
        };
        if self.visible_key.is_some_and(|previous| {
            previous.bounds == key.bounds
                && previous.loops == key.loops
                && previous.revision.same_geometry(key.revision)
        }) {
            self.visible_key = Some(key);
            return;
        }
        self.visible_groups.clear();
        self.visible_nodes.clear();
        self.visible_edges.clear();
        self.advance_stamp();
        self.index.query(bounds, |entry| match entry {
            Indexed::Group(index) => self.visible_groups.push(index as usize),
            Indexed::Node(index) => self.visible_nodes.push(index as usize),
            Indexed::Edge { edge, .. } => {
                let index = edge as usize;
                if self.edge_stamps[index] != self.stamp {
                    self.edge_stamps[index] = self.stamp;
                    if loops || !graph.edge(index).is_loop {
                        self.visible_edges.push(index);
                    }
                }
            }
        });
        self.visible_groups.sort_unstable();
        self.visible_nodes.sort_unstable();
        self.visible_edges.sort_unstable();
        self.visible_key = Some(key);
    }

    fn advance_stamp(&mut self) {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.edge_stamps.fill(0);
            self.node_stamps.fill(0);
            self.group_stamps.fill(0);
            self.stamp = 1;
        }
    }

    pub fn pick(
        &self,
        point: crate::HitPoint,
        zoom: f32,
        loops: bool,
        graph: &(impl Graph + ?Sized),
    ) -> Option<Selection> {
        let mut node_hit = None;
        let mut group_hit = None;
        let mut edge_hit: Option<(usize, crate::EdgeId, f64)> = None;
        self.index.query(
            Rect::from_center_size(
                point.rounded(),
                Vec2::splat(2.0 * (self.max_screen_margin / zoom + point.rounding_error())),
            ),
            |entry| match entry {
                Indexed::Group(index) => {
                    let index = index as usize;
                    if graph.group(index).hit_test(point, zoom) {
                        group_hit =
                            Some(group_hit.map_or(index, |previous: usize| previous.max(index)));
                    }
                }
                Indexed::Node(index) => {
                    let index = index as usize;
                    if graph.node_hit_test(index, point, zoom) {
                        node_hit =
                            Some(node_hit.map_or(index, |previous: usize| previous.max(index)));
                    }
                }
                Indexed::Edge { edge, chunk } if node_hit.is_none() => {
                    let index = edge as usize;
                    let radius = graph.edge_pick_radius(index) / zoom;
                    let edge = graph.edge(index);
                    if !loops && edge.is_loop {
                        return;
                    }
                    let Some(distance) =
                        edge.path
                            .chunk_hit_distance_sq_precise(chunk as usize, point, radius)
                    else {
                        return;
                    };
                    if edge_hit.is_none_or(|(_, previous, best)| {
                        distance < best || (distance == best && edge.id < previous)
                    }) {
                        edge_hit = Some((index, edge.id, distance));
                    }
                }
                Indexed::Edge { .. } => {}
            },
        );
        node_hit
            .map(Selection::Node)
            .or_else(|| edge_hit.map(|(index, _, _)| Selection::Edge(index)))
            .or_else(|| group_hit.map(Selection::Group))
    }
}

impl Canvas {
    pub fn stats(&self) -> CanvasStats {
        CanvasStats {
            snap_rebuilds: self.snapping.rebuilds,
            crossing_rebuilds: self.crossings.full_rebuilds,
            crossing_scanned_edges: self.crossings.scanned_edges,
            ..self.spatial.stats()
        }
    }
}
fn slot_heap_bytes(slots: &SmallVec<[u32; 8]>) -> usize {
    if slots.spilled() {
        slots.capacity() * size_of::<u32>()
    } else {
        0
    }
}

fn edge_margin(graph: &(impl Graph + ?Sized), index: usize) -> Result<f32, SpatialError> {
    let radius = graph.edge_pick_radius(index);
    if !radius.is_finite() || radius < 0.0 {
        return Err(SpatialError::InvalidBounds);
    }
    let margin = graph.edge_screen_margin(index);
    if !margin.is_finite() || margin < 0.0 {
        return Err(SpatialError::InvalidBounds);
    }
    Ok(margin.max(radius))
}

fn group_margin(graph: &(impl Graph + ?Sized), index: usize) -> Result<f32, SpatialError> {
    let margin = graph.group_screen_margin(index);
    if !margin.is_finite() || margin < 0.0 {
        return Err(SpatialError::InvalidBounds);
    }
    Ok(margin.max(crate::groups::PICK_MARGIN))
}

fn node_bounds(graph: &(impl Graph + ?Sized), index: usize) -> Result<(Rect, f32), SpatialError> {
    let node = graph.node(index);
    let geometry = graph.node_geometry(index);
    let margin = graph
        .node_screen_margin(index)
        .max(geometry.screen_margin() + 32.0);
    if !margin.is_finite() || margin < 0.0 {
        return Err(SpatialError::InvalidBounds);
    }
    Ok((geometry.world_bounds(node.position), margin))
}

#[cfg(test)]
mod tests;
