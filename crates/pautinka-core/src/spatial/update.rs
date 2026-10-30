use emath::Rect;

use super::{SpatialError, SpatialHandle, SpatialIndex, valid_bounds};

/// Способ восстановления BVH после изменения границ записей.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialUpdateMode {
    /// Редкие изменения переустанавливают листья и сразу обновляют их предков.
    Incremental,
    /// Массовые изменения завершаются одним обходом дерева снизу вверх, без аллокаций при сохранении пространственной локальности.
    Refit,
}

/// Исключительное заимствование не позволяет читать индекс с ещё не обновлёнными предками.
pub struct SpatialUpdates<'a, K> {
    index: &'a mut SpatialIndex<K>,
    mode: SpatialUpdateMode,
    changed: bool,
}

impl<'a, K> SpatialUpdates<'a, K> {
    pub(super) fn new(index: &'a mut SpatialIndex<K>, mode: SpatialUpdateMode) -> Self {
        Self {
            index,
            mode,
            changed: false,
        }
    }

    pub fn handle(&self, ordinal: usize) -> Option<SpatialHandle> {
        self.index.handle(ordinal)
    }

    pub fn update(&mut self, handle: SpatialHandle, bounds: Rect) -> Result<(), SpatialError> {
        if self.mode == SpatialUpdateMode::Incremental {
            return self.index.update(handle, bounds);
        }
        if !valid_bounds(bounds) {
            return Err(SpatialError::InvalidBounds);
        }
        let leaf = self.index.resolve(handle)?;
        let previous = &mut self.index.nodes[leaf.offset()].bounds;
        if *previous != bounds {
            *previous = bounds;
            self.changed = true;
        }
        Ok(())
    }

    pub fn insert(&mut self, value: K, bounds: Rect) -> Result<SpatialHandle, SpatialError> {
        self.changed = true;
        self.index.insert(value, bounds)
    }
}

impl<K: Copy> SpatialUpdates<'_, K> {
    pub fn remove(&mut self, handle: SpatialHandle) -> Result<K, SpatialError> {
        self.changed = true;
        self.index.remove(handle)
    }
}

impl<K> Drop for SpatialUpdates<'_, K> {
    fn drop(&mut self) {
        if self.mode == SpatialUpdateMode::Refit && self.changed {
            self.index.finish_updates();
        }
    }
}
