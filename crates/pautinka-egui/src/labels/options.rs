use std::num::NonZeroU8;

use egui::{Color32, Rect, Vec2};

/// Положение относительно экранных границ узла; размеры текста не зависят от мирового масштаба.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum LabelPosition {
    #[default]
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    North,
}

impl LabelPosition {
    pub const ALL: [Self; 8] = [
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
        Self::North,
    ];

    pub(super) fn rect(self, node: Rect, size: Vec2, gap: f32) -> Rect {
        let x = match self {
            Self::NorthEast | Self::East | Self::SouthEast => node.right() + gap,
            Self::NorthWest | Self::West | Self::SouthWest => node.left() - gap - size.x,
            Self::North | Self::South => node.center().x - size.x * 0.5,
        };
        let y = match self {
            Self::NorthWest | Self::North | Self::NorthEast => node.top() - gap - size.y,
            Self::SouthWest | Self::South | Self::SouthEast => node.bottom() + gap,
            Self::East | Self::West => node.center().y - size.y * 0.5,
        };
        Rect::from_min_size(egui::pos2(x, y), size)
    }

    pub(super) fn distance(self, other: Self) -> f64 {
        let distance = (self as u8).abs_diff(other as u8);
        f64::from(distance.min(8 - distance)) / 4.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelPlacement {
    Auto {
        allowed: LabelPositions,
        preferred: LabelPosition,
    },
    /// Положение закреплено; возможность скрытия по-прежнему задаёт `LabelVisibility`.
    Fixed(LabelPosition),
}

impl LabelPlacement {
    pub(super) fn positions(self) -> LabelPositions {
        match self {
            Self::Auto { allowed, .. } => allowed,
            Self::Fixed(position) => LabelPositions::new(position),
        }
    }
    pub(super) fn preferred(self) -> LabelPosition {
        match self {
            Self::Auto { preferred, .. } | Self::Fixed(preferred) => preferred,
        }
    }
}

impl Default for LabelPlacement {
    fn default() -> Self {
        Self::Auto {
            allowed: LabelPositions::ALL,
            preferred: LabelPosition::NorthEast,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelVisibility {
    #[default]
    Auto,
    /// Обходит бюджет, пороги масштаба и LOD; при конфликтах выбирается наименее плохая позиция.
    Always,
    /// Скрывает подпись даже при выборе или наведении.
    Hidden,
}

/// Непустой набор кандидатов без аллокаций; порядок обхода всегда одинаковый.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelPositions(NonZeroU8);

impl LabelPositions {
    pub const ALL: Self = Self(NonZeroU8::MAX);

    pub const fn new(position: LabelPosition) -> Self {
        Self(NonZeroU8::new(1 << position as u8).unwrap())
    }
    pub const fn with(self, position: LabelPosition) -> Self {
        Self(NonZeroU8::new(self.0.get() | (1 << position as u8)).unwrap())
    }
    pub const fn contains(self, position: LabelPosition) -> bool {
        self.0.get() & (1 << position as u8) != 0
    }

    /// Перечисляет только разрешённые позиции в порядке `LabelPosition::ALL`.
    pub fn iter(self) -> impl Iterator<Item = LabelPosition> {
        let mut bits = self.0.get();
        std::iter::from_fn(move || {
            if bits == 0 {
                return None;
            }
            let index = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            Some(LabelPosition::ALL[index])
        })
    }
}

impl Default for LabelPositions {
    fn default() -> Self {
        Self::ALL
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeLabelOptions {
    /// Большие значения размещаются раньше; равенство разрешается по стабильному `NodeId`.
    pub priority: i32,
    pub placement: LabelPlacement,
    pub visibility: LabelVisibility,
}

/// Какие состояния требуют показа независимо от масштаба, LOD и бюджета.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LabelEmphasis {
    pub selected: bool,
    pub hovered: bool,
    pub highlighted: bool,
}

impl LabelEmphasis {
    pub(super) fn importance(self, state: crate::ElementState) -> u8 {
        if state.selected && self.selected {
            3
        } else if state.hovered && self.hovered {
            2
        } else {
            u8::from(state.highlighted && self.highlighted)
        }
    }
}

impl Default for LabelEmphasis {
    fn default() -> Self {
        Self {
            selected: true,
            hovered: true,
            highlighted: true,
        }
    }
}

/// Стоимость пересечения — доля площади подписи, умноженная на вес; пересечения суммируются.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelWeights {
    pub labels: f32,
    pub nodes: f32,
    pub viewport: f32,
    /// Штраф за отклонение от предпочтительного направления, от нуля до этого значения.
    pub preference: f32,
}

impl Default for LabelWeights {
    fn default() -> Self {
        Self {
            labels: 100.0,
            nodes: 20.0,
            viewport: 100.0,
            preference: 0.05,
        }
    }
}

/// Все расстояния заданы в экранных точках; числа должны быть конечными и неотрицательными.
/// `reveal_below <= hide_above` задаёт гистерезис; пороги относятся только к пересечениям, без предпочтения позиции.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelLayoutOptions {
    pub gap: f32,
    /// Минимальный зазор между прямоугольниками подписей.
    pub label_padding: f32,
    pub node_padding: f32,
    /// Общий бюджет; обязательные подписи могут его превышать.
    pub max_labels: usize,
    pub weights: LabelWeights,
    pub hide_above: f32,
    pub reveal_below: f32,
    /// Предыдущая позиция сохраняется, если выигрыш новой не превышает это значение.
    pub switch_margin: f32,
    pub emphasis: LabelEmphasis,
    /// Подложка обязательных подписей при превышении порога скрытия.
    pub conflict_background: Option<Color32>,
    pub background_padding: f32,
}

impl LabelLayoutOptions {
    pub(crate) fn valid(self) -> bool {
        [
            self.gap,
            self.label_padding,
            self.node_padding,
            self.hide_above,
            self.reveal_below,
            self.switch_margin,
            self.background_padding,
            self.weights.labels,
            self.weights.nodes,
            self.weights.viewport,
            self.weights.preference,
        ]
        .into_iter()
        .all(|value| value.is_finite() && value >= 0.0)
            && self.reveal_below <= self.hide_above
    }
}
impl Default for LabelLayoutOptions {
    fn default() -> Self {
        Self {
            gap: 4.0,
            label_padding: 2.0,
            node_padding: 1.0,
            max_labels: usize::MAX,
            weights: LabelWeights::default(),
            hide_above: 0.0,
            reveal_below: 0.0,
            switch_margin: 0.1,
            emphasis: LabelEmphasis::default(),
            conflict_background: Some(Color32::from_black_alpha(220)),
            background_padding: 2.0,
        }
    }
}
