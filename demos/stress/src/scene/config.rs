use std::ops::RangeInclusive;

use egui::{Color32, Vec2};
use pautinka::egui::{CornerRadius, Direction, NodeShape, RoutingMode};

pub const STAGES: [usize; 8] = [100, 250, 500, 1_000, 2_500, 5_000, 10_000, 20_000];
pub(super) const PALETTE: [Color32; 12] = [
    Color32::from_rgb(91, 207, 204),
    Color32::from_rgb(241, 180, 93),
    Color32::from_rgb(159, 155, 255),
    Color32::from_rgb(246, 130, 159),
    Color32::from_rgb(127, 211, 134),
    Color32::from_rgb(104, 182, 255),
    Color32::from_rgb(221, 158, 245),
    Color32::from_rgb(246, 214, 111),
    Color32::from_rgb(241, 144, 100),
    Color32::from_rgb(156, 216, 224),
    Color32::from_rgb(206, 221, 154),
    Color32::from_rgb(224, 203, 231),
];

/// Соотношение рёбер к вершинам хранится в сотых долях, чтобы число рёбер не зависело от округления float в генераторе.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EdgeRatio(u16);

impl EdgeRatio {
    const SCALE: usize = 100;

    fn parse(value: f64) -> Result<Self, &'static str> {
        if !Load::EDGES_PER_NODE.contains(&value) {
            return Err("Expected an edge/node ratio in 0..=16");
        }
        let units = (value * Self::SCALE as f64).round() as i32;
        Ok(Self(
            u16::try_from(units).expect("Validated ratio must fit in u16"),
        ))
    }

    fn offset(self, node: usize) -> usize {
        (node * usize::from(self.0) + Self::SCALE / 2) / Self::SCALE
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(try_from = "usize")]
pub(crate) struct NodeCount(usize);

impl NodeCount {
    pub const MIN: Self = Self(STAGES[0]);
    pub const MAX: Self = Self(STAGES[STAGES.len() - 1]);

    pub const fn get(self) -> usize {
        self.0
    }
}

impl TryFrom<usize> for NodeCount {
    type Error = &'static str;

    fn try_from(nodes: usize) -> Result<Self, Self::Error> {
        if !Load::NODES.contains(&nodes) {
            return Err("Expected 100..=20000 nodes");
        }
        Ok(Self(nodes))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Load {
    nodes: NodeCount,
    ratio: EdgeRatio,
}

impl Load {
    pub const NODES: RangeInclusive<usize> = NodeCount::MIN.get()..=NodeCount::MAX.get();
    pub const MAX_EDGES_PER_NODE: u8 = 16;
    pub const EDGES_PER_NODE: RangeInclusive<f64> = 0.0..=Self::MAX_EDGES_PER_NODE as f64;

    pub fn new(nodes: usize, edges_per_node: impl Into<f64>) -> Result<Self, &'static str> {
        Ok(Self {
            nodes: nodes.try_into()?,
            ratio: EdgeRatio::parse(edges_per_node.into())?,
        })
    }
    pub fn nodes(self) -> usize {
        self.nodes.get()
    }
    pub fn edges_per_node(self) -> f64 {
        f64::from(self.ratio.0) / EdgeRatio::SCALE as f64
    }
    pub fn edge_count(self) -> usize {
        self.ratio.offset(self.nodes())
    }

    pub(crate) fn node_count(self) -> NodeCount {
        self.nodes
    }

    pub(crate) fn with_node_count(self, nodes: NodeCount) -> Self {
        Self { nodes, ..self }
    }

    /// Плотные диапазоны распределяют дробную часть степени равномерно и сохраняют ID при росте.
    pub(super) fn edge_range(self, node: usize) -> std::ops::Range<usize> {
        self.ratio.offset(node)..self.ratio.offset(node + 1)
    }
    pub(super) fn supports_backbone(self) -> bool {
        self.ratio.0 >= EdgeRatio::SCALE as u16
    }

    pub fn first_stage(self) -> Self {
        self.with_node_count(NodeCount::MIN)
    }
    pub fn next_stage(self) -> Option<Self> {
        STAGES
            .iter()
            .copied()
            .find(|&nodes| nodes > self.nodes())
            .map(|nodes| self.with_node_count(NodeCount(nodes)))
    }
}

impl Default for Load {
    fn default() -> Self {
        Self {
            nodes: NodeCount::MIN,
            ratio: EdgeRatio(300),
        }
    }
}

/// Вероятности относятся к дополнительным рёбрам: сначала выбираются хабы, затем локальные или дальние связи.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topology {
    pub(super) locality: u8,
    pub(super) local_radius: u8,
    pub(super) hub_share: u8,
    pub(super) hubs: u8,
    pub(super) backbone: bool,
}

impl Topology {
    pub const PERCENT: RangeInclusive<u8> = 0..=100;
    pub const LOCAL_RADIUS: RangeInclusive<u8> = 1..=16;
    pub const HUBS: RangeInclusive<u8> = 1..=32;
    pub const MIXED: Self = Self {
        locality: 85,
        local_radius: 3,
        hub_share: 0,
        hubs: 4,
        backbone: true,
    };
    pub const LOCAL: Self = Self {
        locality: 100,
        local_radius: 2,
        ..Self::MIXED
    };
    pub const RANDOM: Self = Self {
        locality: 0,
        backbone: false,
        ..Self::MIXED
    };
    pub const HUB: Self = Self {
        locality: 70,
        hub_share: 80,
        ..Self::MIXED
    };

    pub fn new(
        locality: u8,
        local_radius: u8,
        hub_share: u8,
        hubs: u8,
        backbone: bool,
    ) -> Result<Self, &'static str> {
        if !Self::PERCENT.contains(&locality)
            || !Self::LOCAL_RADIUS.contains(&local_radius)
            || !Self::PERCENT.contains(&hub_share)
            || !Self::HUBS.contains(&hubs)
        {
            return Err("Expected percentages in 0..=100, radius in 1..=16 and hubs in 1..=32");
        }
        Ok(Self {
            locality,
            local_radius,
            hub_share,
            hubs,
            backbone,
        })
    }
    pub fn locality(self) -> u8 {
        self.locality
    }
    pub fn local_radius(self) -> u8 {
        self.local_radius
    }
    pub fn hub_share(self) -> u8 {
        self.hub_share
    }
    pub fn hubs(self) -> u8 {
        self.hubs
    }
    pub fn backbone(self) -> bool {
        self.backbone
    }
}

impl Default for Topology {
    fn default() -> Self {
        Self::MIXED
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Circle,
    Rectangle,
    Diamond,
    Triangle,
    Hexagon,
    #[default]
    Mixed,
}

impl NodeKind {
    pub(super) fn shape_index(self, index: usize) -> usize {
        match self {
            Self::Circle => 0,
            Self::Rectangle => 1,
            Self::Diamond => 2,
            Self::Triangle => 3,
            Self::Hexagon => 4,
            Self::Mixed => index % 5,
        }
    }
}

/// Настройки без производных массивов: можно дёшево копировать при каждом показе UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    pub kind: NodeKind,
    colors: u8,
    radius: f32,
}

impl Appearance {
    pub const COLORS: RangeInclusive<u8> = 1..=PALETTE.len() as u8;
    pub const RADIUS: RangeInclusive<f32> = 3.0..=16.0;

    pub fn new(kind: NodeKind, colors: u8, radius: f32) -> Result<Self, &'static str> {
        if !Self::COLORS.contains(&colors) || !Self::RADIUS.contains(&radius) {
            return Err("Expected 1..=12 colors and a radius in 3..=16");
        }
        Ok(Self {
            kind,
            colors,
            radius,
        })
    }
    pub fn colors(self) -> u8 {
        self.colors
    }
    pub fn radius(self) -> f32 {
        self.radius
    }
    pub(super) fn color(self, index: usize) -> Color32 {
        PALETTE[index % usize::from(self.colors)]
    }
    pub(super) fn shapes(self) -> [NodeShape; 5] {
        let size = Vec2::splat(self.radius * 2.0);
        [
            NodeShape::circle(self.radius),
            NodeShape::rectangle(size),
            NodeShape::diamond(size),
            NodeShape::triangle(size),
            NodeShape::regular_polygon(self.radius, 6),
        ]
        .map(|shape| shape.expect("Validated radius must produce a valid shape"))
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            kind: NodeKind::Mixed,
            colors: 6,
            radius: 7.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneStyle {
    pub appearance: Appearance,
    pub routing: RoutingMode,
    pub corner_radius: CornerRadius,
    pub direction: Direction,
}

impl Default for SceneStyle {
    fn default() -> Self {
        Self {
            appearance: Appearance::default(),
            routing: RoutingMode::Curved,
            corner_radius: CornerRadius::default(),
            direction: Direction::Undirected,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub(super) fraction: u8,
    pub(super) speed: f32,
}

impl Motion {
    pub const FRACTION: RangeInclusive<u8> = 0..=100;
    pub const SPEED: RangeInclusive<f32> = 0.0..=3.0;

    pub fn new(fraction: u8, speed: f32) -> Result<Self, &'static str> {
        if !Self::FRACTION.contains(&fraction) || !Self::SPEED.contains(&speed) {
            return Err("Expected a moving fraction in 0..=100 and speed in 0..=3");
        }
        Ok(Self { fraction, speed })
    }
    pub fn fraction(self) -> u8 {
        self.fraction
    }
    pub fn speed(self) -> f32 {
        self.speed
    }
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            fraction: 100,
            speed: 1.0,
        }
    }
}
