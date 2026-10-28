use std::{
    error::Error,
    fmt,
    num::NonZeroU32,
    sync::atomic::{AtomicU64, Ordering},
};

use emath::Rect;

static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialError {
    InvalidBounds,
    TooManyEntries,
    InvalidHandle,
}

impl Error for SpatialError {}

impl fmt::Display for SpatialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidBounds => "Spatial bounds must be finite and ordered",
            Self::TooManyEntries => "Spatial index exceeds the supported entry count",
            Self::InvalidHandle => "Spatial handle belongs to another index or revision",
        })
    }
}

#[derive(Clone, Copy)]
enum Children {
    Pending,
    Leaf(u32),
    Branch(TreeIndex, TreeIndex),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpatialHandle {
    owner: u64,
    generation: u64,
    entry: u32,
    slot_generation: u64,
}

impl SpatialHandle {
    /// Номер временного слота; повторное использование проверяется полным handle.
    pub fn slot(self) -> u32 {
        self.entry
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TreeIndex(NonZeroU32);

impl TreeIndex {
    fn from_offset(index: usize) -> Self {
        Self(
            NonZeroU32::new(u32::try_from(index + 1).expect("Prechecked spatial index size"))
                .unwrap(),
        )
    }
    fn offset(self) -> usize {
        self.0.get() as usize - 1
    }
}

struct TreeNode {
    bounds: Rect,
    parent: Option<TreeIndex>,
    children: Children,
    height: u16,
}

struct BuildEntry {
    ordinal: u32,
    bounds: Rect,
}

/// Сбалансированный BVH с локальным обновлением границ без аллокаций.
///
/// Дубликаты значений допустимы: несколько сегментов могут ссылаться на одно ребро.
/// Порядок обхода не является порядком рисования. Перестройка инвалидирует все handles.
/// Перемещения переустанавливают листья и балансируют дерево. Освобождённые узлы и слоты переиспользуются.
pub struct SpatialIndex<K> {
    owner: u64,
    generation: u64,
    nodes: Vec<TreeNode>,
    values: Vec<K>,
    leaves: Vec<Option<TreeIndex>>,
    slot_generations: Vec<u64>,
    free_entries: Vec<u32>,
    free_nodes: Vec<TreeIndex>,
    root: Option<TreeIndex>,
    count: usize,
}

impl<K> SpatialIndex<K> {
    /// Общие мировые границы живых записей, без сканирования дерева.
    pub fn total_bounds(&self) -> Option<Rect> {
        self.root.map(|root| self.nodes[root.offset()].bounds)
    }
    pub fn len(&self) -> usize {
        self.count
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn storage_bytes(&self) -> usize {
        self.nodes.capacity() * size_of::<TreeNode>()
            + self.values.capacity() * size_of::<K>()
            + self.leaves.capacity() * size_of::<Option<TreeIndex>>()
            + self.slot_generations.capacity() * size_of::<u64>()
            + self.free_entries.capacity() * size_of::<u32>()
            + self.free_nodes.capacity() * size_of::<TreeIndex>()
    }

    /// Некорректный ввод оставляет предыдущий индекс действующим.
    pub fn rebuild(
        &mut self,
        entries: impl IntoIterator<Item = (K, Rect)>,
    ) -> Result<(), SpatialError> {
        let entries: Vec<_> = entries.into_iter().collect();
        if entries.len() > u32::MAX as usize / 2 {
            return Err(SpatialError::TooManyEntries);
        }
        if entries.iter().any(|(_, bounds)| !valid_bounds(*bounds)) {
            return Err(SpatialError::InvalidBounds);
        }
        self.generation = self
            .generation
            .checked_add(1)
            .expect("Spatial index generations exhausted");
        self.nodes.clear();
        self.values.clear();
        self.leaves.clear();
        self.slot_generations.clear();
        self.free_entries.clear();
        self.free_nodes.clear();
        self.root = None;
        self.count = entries.len();
        self.nodes
            .reserve(entries.len().saturating_mul(2).saturating_add(128));
        self.values.reserve(entries.len().saturating_add(64));
        self.leaves.resize(entries.len(), None);
        self.slot_generations.resize(entries.len(), 0);
        self.free_entries.reserve(64);
        self.free_nodes.reserve(64);
        let mut build = Vec::with_capacity(entries.len());
        for (ordinal, (value, bounds)) in entries.into_iter().enumerate() {
            self.values.push(value);
            build.push(BuildEntry {
                ordinal: ordinal as u32,
                bounds,
            });
        }
        if !build.is_empty() {
            self.root = Some(self.build(&mut build, None));
        }
        Ok(())
    }

    fn build(&mut self, entries: &mut [BuildEntry], parent: Option<TreeIndex>) -> TreeIndex {
        let bounds = entries
            .iter()
            .fold(Rect::NOTHING, |bounds, entry| bounds.union(entry.bounds));
        let index = self.allocate_node(bounds, Children::Pending);
        self.nodes[index.offset()].parent = parent;
        let children = if let [entry] = entries {
            self.leaves[entry.ordinal as usize] = Some(index);
            Children::Leaf(entry.ordinal)
        } else {
            let horizontal = bounds.width() >= bounds.height();
            let middle = entries.len() / 2;
            entries.select_nth_unstable_by(middle, |a, b| {
                let center = |entry: &BuildEntry| {
                    if horizontal {
                        entry.bounds.min.x * 0.5 + entry.bounds.max.x * 0.5
                    } else {
                        entry.bounds.min.y * 0.5 + entry.bounds.max.y * 0.5
                    }
                };
                center(a)
                    .total_cmp(&center(b))
                    .then_with(|| a.ordinal.cmp(&b.ordinal))
            });
            let (left, right) = entries.split_at_mut(middle);
            Children::Branch(
                self.build(left, Some(index)),
                self.build(right, Some(index)),
            )
        };
        self.nodes[index.offset()].children = children;
        self.recalculate(index);
        index
    }

    pub fn handle(&self, ordinal: usize) -> Option<SpatialHandle> {
        self.leaves
            .get(ordinal)
            .copied()
            .flatten()
            .map(|_| SpatialHandle {
                owner: self.owner,
                generation: self.generation,
                entry: ordinal as u32,
                slot_generation: self.slot_generations[ordinal],
            })
    }

    pub fn bounds(&self, handle: SpatialHandle) -> Result<Rect, SpatialError> {
        let leaf = self.resolve(handle)?;
        Ok(self.nodes[leaf.offset()].bounds)
    }

    pub fn update(&mut self, handle: SpatialHandle, bounds: Rect) -> Result<(), SpatialError> {
        if !valid_bounds(bounds) {
            return Err(SpatialError::InvalidBounds);
        }
        let leaf = self.resolve(handle)?;
        if self.nodes[leaf.offset()].bounds == bounds {
            return Ok(());
        }
        if let Some(parent) = self.nodes[leaf.offset()].parent
            && self.nodes[parent.offset()].bounds.contains_rect(bounds)
        {
            self.nodes[leaf.offset()].bounds = bounds;
            self.refit(Some(parent));
            return Ok(());
        }
        self.detach_leaf(leaf);
        self.nodes[leaf.offset()].bounds = bounds;
        self.insert_leaf(leaf);
        Ok(())
    }

    fn resolve(&self, handle: SpatialHandle) -> Result<TreeIndex, SpatialError> {
        if handle.owner != self.owner || handle.generation != self.generation {
            return Err(SpatialError::InvalidHandle);
        }
        let ordinal = handle.entry as usize;
        if self.slot_generations.get(ordinal) != Some(&handle.slot_generation) {
            return Err(SpatialError::InvalidHandle);
        }
        self.leaves
            .get(ordinal)
            .copied()
            .flatten()
            .ok_or(SpatialError::InvalidHandle)
    }

    fn recalculate(&mut self, index: TreeIndex) {
        if let Children::Branch(left, right) = self.nodes[index.offset()].children {
            let bounds = self.nodes[left.offset()]
                .bounds
                .union(self.nodes[right.offset()].bounds);
            let height = 1 + self.nodes[left.offset()]
                .height
                .max(self.nodes[right.offset()].height);
            self.nodes[index.offset()].bounds = bounds;
            self.nodes[index.offset()].height = height;
        }
    }

    fn replace_child(&mut self, parent: Option<TreeIndex>, old: TreeIndex, new: TreeIndex) {
        if let Some(parent) = parent {
            let Children::Branch(left, right) = self.nodes[parent.offset()].children else {
                unreachable!("Spatial parent must be a branch");
            };
            self.nodes[parent.offset()].children = if left == old {
                Children::Branch(new, right)
            } else {
                debug_assert!(right == old);
                Children::Branch(left, new)
            };
        } else {
            self.root = Some(new);
        }
        self.nodes[new.offset()].parent = parent;
    }

    fn set_branch(&mut self, parent: TreeIndex, left: TreeIndex, right: TreeIndex) {
        self.nodes[parent.offset()].children = Children::Branch(left, right);
        self.nodes[left.offset()].parent = Some(parent);
        self.nodes[right.offset()].parent = Some(parent);
        self.recalculate(parent);
    }

    fn balance(&mut self, index: TreeIndex) -> TreeIndex {
        let Children::Branch(left, right) = self.nodes[index.offset()].children else {
            return index;
        };
        let difference = i32::from(self.nodes[right.offset()].height)
            - i32::from(self.nodes[left.offset()].height);
        let tall = if difference > 1 {
            right
        } else if difference < -1 {
            left
        } else {
            return index;
        };
        let Children::Branch(a, b) = self.nodes[tall.offset()].children else {
            unreachable!("Tall child must be a branch");
        };
        let (upper, lower) = if self.nodes[a.offset()].height > self.nodes[b.offset()].height {
            (a, b)
        } else {
            (b, a)
        };
        self.replace_child(self.nodes[index.offset()].parent, index, tall);
        if tall == right {
            self.set_branch(index, left, lower);
            self.set_branch(tall, index, upper);
        } else {
            self.set_branch(index, lower, right);
            self.set_branch(tall, upper, index);
        }
        tall
    }

    fn refit(&mut self, mut next: Option<TreeIndex>) {
        while let Some(index) = next {
            let previous_bounds = self.nodes[index.offset()].bounds;
            let previous_height = self.nodes[index.offset()].height;
            self.recalculate(index);
            let balanced = self.balance(index);
            let node = &self.nodes[balanced.offset()];
            if balanced == index && node.bounds == previous_bounds && node.height == previous_height
            {
                break;
            }
            next = node.parent;
        }
    }

    fn free_node(&mut self, index: TreeIndex) {
        // Остальные поля целиком перезаписывает allocate_node до повторного использования.
        self.nodes[index.offset()].children = Children::Pending;
        self.free_nodes.push(index);
    }

    fn allocate_node(&mut self, bounds: Rect, children: Children) -> TreeIndex {
        let node = TreeNode {
            bounds,
            children,
            parent: None,
            height: 0,
        };
        if let Some(index) = self.free_nodes.pop() {
            self.nodes[index.offset()] = node;
            index
        } else {
            let index = TreeIndex::from_offset(self.nodes.len());
            self.nodes.push(node);
            index
        }
    }

    fn detach_leaf(&mut self, leaf: TreeIndex) {
        let Some(parent) = self.nodes[leaf.offset()].parent else {
            self.root = None;
            return;
        };
        let Children::Branch(left, right) = self.nodes[parent.offset()].children else {
            unreachable!("Spatial leaf has an invalid parent");
        };
        let sibling = if left == leaf { right } else { left };
        let grandparent = self.nodes[parent.offset()].parent;
        self.replace_child(grandparent, parent, sibling);
        self.nodes[leaf.offset()].parent = None;
        self.free_node(parent);
        self.refit(grandparent);
    }

    fn insert_leaf(&mut self, leaf: TreeIndex) {
        let Some(mut sibling) = self.root else {
            self.root = Some(leaf);
            self.nodes[leaf.offset()].parent = None;
            return;
        };
        let bounds = self.nodes[leaf.offset()].bounds;
        while let Children::Branch(left, right) = self.nodes[sibling.offset()].children {
            let cost = |index: TreeIndex| {
                let old = self.nodes[index.offset()].bounds;
                perimeter(old.union(bounds)) - perimeter(old)
            };
            let left_cost = cost(left);
            let right_cost = cost(right);
            sibling = if left_cost < right_cost
                || (left_cost == right_cost
                    && self.nodes[left.offset()].height <= self.nodes[right.offset()].height)
            {
                left
            } else {
                right
            };
        }
        let parent = self.nodes[sibling.offset()].parent;
        let branch = self.allocate_node(
            bounds.union(self.nodes[sibling.offset()].bounds),
            Children::Pending,
        );
        self.replace_child(parent, sibling, branch);
        self.set_branch(branch, sibling, leaf);
        // Новый узел уже пересчитан в set_branch; изменение ещё не учтено только его предками.
        self.refit(parent);
    }

    pub fn insert(&mut self, value: K, bounds: Rect) -> Result<SpatialHandle, SpatialError> {
        if !valid_bounds(bounds) {
            return Err(SpatialError::InvalidBounds);
        }
        let extra = if self.root.is_some() { 2usize } else { 1 };
        if self.count >= u32::MAX as usize / 2
            || self.values.len() >= u32::MAX as usize
            || self
                .nodes
                .len()
                .saturating_add(extra.saturating_sub(self.free_nodes.len()))
                > u32::MAX as usize
        {
            return Err(SpatialError::TooManyEntries);
        }
        let ordinal = if let Some(ordinal) = self.free_entries.pop() {
            self.values[ordinal as usize] = value;
            ordinal as usize
        } else {
            let ordinal = self.values.len();
            self.values.push(value);
            self.leaves.push(None);
            self.slot_generations.push(0);
            ordinal
        };
        let leaf = self.allocate_node(bounds, Children::Leaf(ordinal as u32));
        self.leaves[ordinal] = Some(leaf);
        self.insert_leaf(leaf);
        self.count += 1;
        Ok(self.handle(ordinal).unwrap())
    }
}

impl<K: Copy> SpatialIndex<K> {
    pub fn remove(&mut self, handle: SpatialHandle) -> Result<K, SpatialError> {
        let leaf = self.resolve(handle)?;
        let ordinal = handle.entry as usize;
        self.detach_leaf(leaf);
        self.free_node(leaf);
        self.leaves[ordinal] = None;
        self.slot_generations[ordinal] = self.slot_generations[ordinal]
            .checked_add(1)
            .expect("Spatial slot generations exhausted");
        self.free_entries.push(handle.entry);
        self.count -= 1;
        Ok(self.values[ordinal])
    }

    /// Обход использует только стек; глубина дерева ограничена сбалансированной постройкой.
    pub fn query(&self, bounds: Rect, mut visit: impl FnMut(K)) {
        if valid_bounds(bounds)
            && let Some(root) = self.root
        {
            self.query_node(root, bounds, &mut visit);
        }
    }

    /// Обходит каждую неупорядоченную пару записей с пересекающимися границами ровно один раз.
    ///
    /// Касания включаются; запись не образует пару сама с собой. Порядок значений в паре произволен.
    /// Обход использует только стек и не выделяет память в куче.
    pub fn query_pairs(&self, mut visit: impl FnMut(K, K)) {
        if let Some(root) = self.root {
            self.query_pair_nodes(root, root, &mut visit);
        }
    }

    fn query_node(&self, index: TreeIndex, bounds: Rect, visit: &mut impl FnMut(K)) {
        let node = &self.nodes[index.offset()];
        if !node.bounds.intersects(bounds) {
            return;
        }
        match node.children {
            Children::Leaf(ordinal) => visit(self.values[ordinal as usize]),
            Children::Branch(left, right) => {
                self.query_node(left, bounds, visit);
                self.query_node(right, bounds, visit);
            }
            Children::Pending => unreachable!("Incomplete spatial tree"),
        }
    }

    fn query_pair_nodes(&self, a: TreeIndex, b: TreeIndex, visit: &mut impl FnMut(K, K)) {
        let first = &self.nodes[a.offset()];
        if a == b {
            match first.children {
                Children::Branch(left, right) => {
                    self.query_pair_nodes(left, left, visit);
                    self.query_pair_nodes(left, right, visit);
                    self.query_pair_nodes(right, right, visit);
                }
                Children::Leaf(_) => {}
                Children::Pending => unreachable!("Incomplete spatial tree"),
            }
            return;
        }
        let second = &self.nodes[b.offset()];
        if !first.bounds.intersects(second.bounds) {
            return;
        }
        let (taller, other_index, other) = if first.height >= second.height {
            (first, b, second)
        } else {
            (second, a, first)
        };
        match taller.children {
            Children::Branch(left, right) => {
                self.query_pair_nodes(left, other_index, visit);
                self.query_pair_nodes(right, other_index, visit);
            }
            Children::Leaf(left) => {
                let Children::Leaf(right) = other.children else {
                    unreachable!("Leaf cannot be taller than a branch")
                };
                visit(self.values[left as usize], self.values[right as usize]);
            }
            Children::Pending => unreachable!("Incomplete spatial tree"),
        }
    }
}

impl<K> Default for SpatialIndex<K> {
    fn default() -> Self {
        Self {
            owner: NEXT_OWNER
                .try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .expect("Spatial index identities exhausted"),
            generation: 0,
            nodes: Vec::new(),
            values: Vec::new(),
            leaves: Vec::new(),
            slot_generations: Vec::new(),
            free_entries: Vec::new(),
            free_nodes: Vec::new(),
            root: None,
            count: 0,
        }
    }
}

fn valid_bounds(bounds: Rect) -> bool {
    bounds.is_finite() && bounds.min.x <= bounds.max.x && bounds.min.y <= bounds.max.y
}

fn perimeter(bounds: Rect) -> f64 {
    ((f64::from(bounds.max.x) - f64::from(bounds.min.x))
        + (f64::from(bounds.max.y) - f64::from(bounds.min.y)))
        * 2.0
}

#[cfg(test)]
mod tests;
