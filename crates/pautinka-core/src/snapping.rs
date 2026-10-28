mod choice;
mod grid;
mod index;

use std::ops::{BitOr, BitOrAssign};

use emath::Pos2;

use crate::AlignmentSnap;

pub use grid::SnapGrid;
pub use index::SnapIndex;

/// Семейство параллельных направляющих через опорные вершины.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnapKind {
    Horizontal,
    Vertical,
    DiagonalUp,
    DiagonalDown,
}

impl SnapKind {
    pub const ALL: [Self; 4] = [
        Self::Horizontal,
        Self::Vertical,
        Self::DiagonalUp,
        Self::DiagonalDown,
    ];

    pub fn flag(self) -> SnapKinds {
        SnapKinds(1 << self.index())
    }

    pub fn direction(self) -> emath::Vec2 {
        match self {
            Self::Horizontal => emath::Vec2::X,
            Self::Vertical => emath::Vec2::Y,
            Self::DiagonalUp => emath::vec2(1.0, -1.0) * std::f32::consts::FRAC_1_SQRT_2,
            Self::DiagonalDown => emath::vec2(1.0, 1.0) * std::f32::consts::FRAC_1_SQRT_2,
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    fn normal(self) -> [f64; 2] {
        match self {
            Self::Horizontal => [0.0, 1.0],
            Self::Vertical => [1.0, 0.0],
            Self::DiagonalUp => [1.0, 1.0],
            Self::DiagonalDown => [1.0, -1.0],
        }
    }

    fn coordinate(self, point: Pos2) -> f64 {
        let [a, b] = self.normal();
        a * f64::from(point.x) + b * f64::from(point.y)
    }

    fn along(self, point: Pos2) -> f64 {
        let [a, b] = self.normal();
        b * f64::from(point.x) - a * f64::from(point.y)
    }

    fn distance(self, a: Pos2, b: Pos2) -> f64 {
        let scale = match self {
            Self::Horizontal | Self::Vertical => 1.0,
            Self::DiagonalUp | Self::DiagonalDown => std::f64::consts::FRAC_1_SQRT_2,
        };
        (self.coordinate(a) - self.coordinate(b)).abs() * scale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapTarget {
    /// Пересечение регулярной сетки без опорных вершин.
    Grid,
    Guide(SnapGuide),
    Intersection([SnapGuide; 2]),
}

impl SnapTarget {
    pub fn guides(self) -> impl Iterator<Item = SnapGuide> {
        match self {
            Self::Grid => [None, None],
            Self::Guide(guide) => [Some(guide), None],
            Self::Intersection([a, b]) => [Some(a), Some(b)],
        }
        .into_iter()
        .flatten()
    }
}

/// Типизированный набор семейств направляющих; пустой набор отключает цели от опорных вершин.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SnapKinds(u8);

impl SnapKinds {
    pub const NONE: Self = Self(0);
    pub const HORIZONTAL: Self = Self(1);
    pub const VERTICAL: Self = Self(2);
    pub const DIAGONAL_UP: Self = Self(4);
    pub const DIAGONAL_DOWN: Self = Self(8);
    pub const AXES: Self = Self(3);
    pub const DIAGONALS: Self = Self(12);
    pub const ALL: Self = Self(15);

    pub fn contains(self, kinds: Self) -> bool {
        self.0 & kinds.0 == kinds.0
    }
    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }
    pub fn set(&mut self, kinds: Self, enabled: bool) {
        if enabled {
            self.0 |= kinds.0;
        } else {
            self.0 &= !kinds.0;
        }
    }
}

impl BitOr for SnapKinds {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for SnapKinds {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

/// Общая политика: режимы и допуски в координатах вызывающего.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapOptions {
    pub kinds: SnapKinds,
    pub tolerance: AlignmentSnap,
}

impl Default for SnapOptions {
    fn default() -> Self {
        Self {
            kinds: SnapKinds::AXES,
            tolerance: AlignmentSnap::default(),
        }
    }
}

/// Направляющая через исходную позицию опорной вершины.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapGuide {
    pub kind: SnapKind,
    pub source: Pos2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapResult {
    pub position: Pos2,
    pub target: Option<SnapTarget>,
}

impl SnapResult {
    pub fn guides(self) -> impl Iterator<Item = SnapGuide> {
        self.target.into_iter().flat_map(SnapTarget::guides)
    }
}

#[cfg(test)]
mod tests;
