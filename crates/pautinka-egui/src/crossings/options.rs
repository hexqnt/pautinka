#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossingMode {
    None,
    #[default]
    Gap,
    Bridge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CrossingSize {
    Fixed(CrossingPoints),
    /// Толщина обоих рёбер плюс отступ с каждой стороны, с ограничением в 128 точек.
    Auto {
        padding: CrossingPoints,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BridgeSide {
    /// Сверху для преимущественно горизонтальных рёбер, справа для вертикальных; не зависит от направления пути.
    #[default]
    Auto,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossingOrder {
    #[default]
    StableId,
    HorizontalFirst,
    VerticalFirst,
}

/// Размер оформления в экранных точках, от 0 до 128 включительно.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossingPoints(f32);

impl CrossingPoints {
    pub const MAX: f32 = 128.0;

    pub fn new(value: f32) -> Option<Self> {
        (value.is_finite() && (0.0..=Self::MAX).contains(&value)).then_some(Self(value))
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

/// Минимальный угол пересечения в градусах, от 1 до 90 включительно.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossingAngle(f32);

impl CrossingAngle {
    pub fn new(degrees: f32) -> Option<Self> {
        (degrees.is_finite() && (1.0..=90.0).contains(&degrees)).then_some(Self(degrees))
    }

    pub const fn degrees(self) -> f32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossingOptions {
    pub mode: CrossingMode,
    pub size: CrossingSize,
    /// Нулевая высота заменяет мостики обычными разрывами нижних рёбер.
    pub bridge_height: CrossingPoints,
    pub bridge_side: BridgeSide,
    /// Общий порядок задаётся только настройками холста; в переопределении ребра это поле не используется.
    pub order: CrossingOrder,
    pub min_angle: CrossingAngle,
    /// Дополнительный отступ от концов сегментов, изгибов и стрелок.
    pub clearance: CrossingPoints,
}

impl CrossingOptions {
    pub(super) fn effective_mode(self) -> CrossingMode {
        if self.mode == CrossingMode::Bridge && self.bridge_height.get() == 0.0 {
            CrossingMode::Gap
        } else {
            self.mode
        }
    }

    pub(crate) fn margin(self) -> f32 {
        if self.mode == CrossingMode::Bridge {
            self.bridge_height.get()
        } else {
            0.0
        }
    }
}

impl Default for CrossingOptions {
    fn default() -> Self {
        Self {
            mode: CrossingMode::Gap,
            size: CrossingSize::Fixed(CrossingPoints(10.0)),
            bridge_height: CrossingPoints(6.0),
            bridge_side: BridgeSide::Auto,
            order: CrossingOrder::StableId,
            min_angle: CrossingAngle(15.0),
            clearance: CrossingPoints(2.0),
        }
    }
}

/// Переопределение для стабильного `EdgeId`; больший приоритет означает верхнее ребро.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeCrossings {
    pub priority: i32,
    /// None наследует оформление холста; Some позволяет в том числе отключить оформление ребра.
    pub options: Option<CrossingOptions>,
}
