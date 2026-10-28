/// Устойчивые ключи элементов уникальны в пределах одного графа и не зависят от плотных индексов.
macro_rules! identity {
    ($(#[$attribute:meta])* $name:ident) => {
        $(#[$attribute])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(value: u64) -> Self {
                Self(value)
            }
            pub const fn get(self) -> u64 {
                self.0
            }
        }
    };
}

/// Идентичность экземпляра представления в процессе; не сохраняется в проект.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GraphId(u64);

impl GraphId {
    pub fn fresh() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(
            NEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .expect("Graph identities exhausted"),
        )
    }
    pub const fn get(self) -> u64 {
        self.0
    }
}

identity!(NodeId);
identity!(EdgeId);
identity!(GroupId);
identity!(
    /// Устойчивый ключ порта в пределах одного узла.
    PortId
);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementId {
    Node(NodeId),
    Edge(EdgeId),
    Group(GroupId),
}

/// Плотный индекс элемента в текущей топологии; при перестройке разрешается заново по ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementIndex {
    Node(usize),
    Edge(usize),
    Group(usize),
}

/// Монотонные версии сцены: топология инвалидирует геометрию, геометрия — оформление.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphRevision {
    topology: u64,
    geometry: u64,
    appearance: u64,
}

impl GraphRevision {
    pub const fn topology(self) -> u64 {
        self.topology
    }
    pub const fn geometry(self) -> u64 {
        self.geometry
    }
    pub const fn appearance(self) -> u64 {
        self.appearance
    }
    pub const fn same_geometry(self, other: Self) -> bool {
        self.topology == other.topology && self.geometry == other.geometry
    }
    pub fn next_appearance(self) -> Self {
        Self {
            appearance: self
                .appearance
                .checked_add(1)
                .expect("Appearance revision exhausted"),
            ..self
        }
    }
    pub fn next_geometry(self) -> Self {
        Self {
            geometry: self
                .geometry
                .checked_add(1)
                .expect("Geometry revision exhausted"),
            ..self.next_appearance()
        }
    }
    pub fn next_topology(self) -> Self {
        Self {
            topology: self
                .topology
                .checked_add(1)
                .expect("Topology revision exhausted"),
            ..self.next_geometry()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn topology_invalidates_geometry_but_geometry_preserves_topology() {
        let first = GraphRevision::default().next_geometry();
        let next = first.next_topology();
        assert_eq!((first.topology(), first.geometry()), (0, 1));
        assert_eq!((next.topology(), next.geometry()), (1, 2));
        let recolored = next.next_appearance();
        assert!(recolored.same_geometry(next));
        assert_eq!(recolored.appearance(), next.appearance() + 1);
    }
}
