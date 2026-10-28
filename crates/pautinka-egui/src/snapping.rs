use egui::Rect;
use pautinka_core::{SnapGrid, SnapIndex, SnapKinds, SnapOptions, SnapResult, SnapTarget};

use crate::{
    Camera, CanvasOptions, ChangeCoverage, ElementId, Graph, GraphId, GraphRevision, GridSnapMode,
    NodeId, Selection, selection::SelectionState,
};

/// Состояние захвата принадлежит жесту; смена шага сбрасывает только удержание сетки.
#[derive(Default)]
pub(super) struct DragSnap {
    pub result: Option<SnapResult>,
    grid: Option<(GridSnapMode, SnapGrid)>,
}

impl DragSnap {
    pub fn prepare(
        &mut self,
        options: &CanvasOptions,
        camera: Camera,
        viewport: Rect,
    ) -> Option<(SnapOptions, Option<SnapGrid>)> {
        if !options.interaction.snapping {
            *self = Self::default();
            return None;
        }
        let mode = options.grid_snap;
        let grid = options
            .grid
            .unwrap_or_default()
            .snap_grid(mode, camera.zoom, viewport);
        let key = grid.map(|grid| (mode, grid));
        if key != self.grid
            && self
                .result
                .is_some_and(|snap| snap.target == Some(SnapTarget::Grid))
        {
            self.result = None;
        }
        self.grid = key;
        let policy = options.snapping;
        if policy.kinds.is_empty() && grid.is_none() {
            return None;
        }
        Some((
            SnapOptions {
                tolerance: policy.tolerance.scaled(1.0 / f64::from(camera.zoom))?,
                ..policy
            },
            grid,
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct CacheKey {
    graph: GraphId,
    revision: GraphRevision,
    dragged: NodeId,
    selection_epoch: u64,
    kinds: SnapKinds,
}

#[derive(Default)]
pub(super) struct SnapCache {
    pub index: SnapIndex,
    pub rebuilds: u64,
    key: Option<CacheKey>,
    excluded: Vec<bool>,
}

impl SnapCache {
    pub fn synchronize(
        &mut self,
        graph: &(impl Graph + ?Sized),
        dragged: NodeId,
        selection: &SelectionState,
        kinds: SnapKinds,
    ) {
        let key = CacheKey {
            graph: graph.id(),
            revision: graph.revision(),
            dragged,
            selection_epoch: selection.epoch,
            kinds,
        };
        if self.key == Some(key) {
            return;
        }
        let moving_selection = selection.members.contains(&ElementId::Node(dragged));
        let excluded = |index, id| {
            id == dragged
                || (moving_selection
                    && selection.members.contains(&ElementId::Node(id))
                    && graph.node_interaction(index).draggable)
        };
        let reusable = self.key.is_some_and(|old| {
            if old.graph != key.graph
                || old.dragged != key.dragged
                || old.kinds != key.kinds
                || old.selection_epoch != key.selection_epoch
                || old.revision.topology() != key.revision.topology()
            {
                return false;
            }
            // Оформление может менять закрепление; без журнала геометрии выбирается перестройка.
            if old.revision.same_geometry(key.revision) {
                return false;
            }
            let mut source_changed = false;
            let coverage = graph.geometry_changes(old.revision, &mut |element| {
                if let Selection::Node(index) = element {
                    source_changed |= !self.excluded.get(index).copied().unwrap_or(false)
                        || !excluded(index, graph.node(index).id);
                }
            });
            coverage == ChangeCoverage::Complete && !source_changed
        });
        if !reusable {
            self.rebuilds += 1;
            self.excluded.clear();
            self.index.rebuild(
                (0..graph.node_count()).filter_map(|index| {
                    let node = graph.node(index);
                    let excluded = excluded(index, node.id);
                    self.excluded.push(excluded);
                    (!excluded).then_some(node.position)
                }),
                kinds,
            );
        }
        self.key = Some(key);
    }
}

#[cfg(test)]
mod tests;
