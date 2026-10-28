use std::time::Duration;

use pautinka::egui::{CrossingMode, Direction, RoutingMode};
use serde::Deserialize;

use crate::scene::{self, NodeCount};

use super::{Pattern, ViewSettings};

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Scenario {
    pub seed: u32,
    pub run: Run,
    pub load: Load,
    pub growth: Growth,
    pub topology: Topology,
    pub motion: Motion,
    pub nodes: Nodes,
    pub edges: Edges,
    pub view: ViewSettings,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            seed: 42,
            run: Run::default(),
            load: Load::default(),
            growth: Growth::default(),
            topology: Topology::default(),
            motion: Motion::default(),
            nodes: Nodes::default(),
            edges: Edges::default(),
            view: ViewSettings::default(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Run {
    pub duration_secs: Option<PositiveSeconds>,
    pub max_nodes: Option<NodeCount>,
    pub hold_secs: Option<Seconds>,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Load {
    pub nodes: usize,
    pub edges_per_node: f64,
}

impl Default for Load {
    fn default() -> Self {
        let load = scene::Load::default();
        Self {
            nodes: load.nodes(),
            edges_per_node: load.edges_per_node(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Growth {
    pub enabled: bool,
    pub interval_secs: PositiveSeconds,
}

impl Default for Growth {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: PositiveSeconds(Duration::from_secs(10)),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Topology {
    pub locality: u8,
    pub local_radius: u8,
    pub hub_share: u8,
    pub hubs: u8,
    pub backbone: bool,
}

impl Default for Topology {
    fn default() -> Self {
        let topology = scene::Topology::default();
        Self {
            locality: topology.locality(),
            local_radius: topology.local_radius(),
            hub_share: topology.hub_share(),
            hubs: topology.hubs(),
            backbone: topology.backbone(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Motion {
    pub animate: bool,
    pub fraction: u8,
    pub speed: f32,
}

impl Default for Motion {
    fn default() -> Self {
        let motion = scene::Motion::default();
        Self {
            animate: true,
            fraction: motion.fraction(),
            speed: motion.speed(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Nodes {
    pub kind: scene::NodeKind,
    pub colors: u8,
    pub radius: f32,
}

impl Default for Nodes {
    fn default() -> Self {
        let appearance = scene::Appearance::default();
        Self {
            kind: appearance.kind,
            colors: appearance.colors(),
            radius: appearance.radius(),
        }
    }
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Edges {
    #[serde(with = "Routing")]
    pub routing: RoutingMode,
    pub corner_radius: CornerRadius,
    #[serde(with = "Arrows")]
    pub direction: Direction,
    pub pattern: Pattern,
    #[serde(with = "Crossings")]
    pub crossings: CrossingMode,
}

impl Default for Edges {
    fn default() -> Self {
        let style = scene::SceneStyle::default();
        Self {
            routing: style.routing,
            corner_radius: CornerRadius(style.corner_radius),
            direction: style.direction,
            pattern: Pattern::default(),
            crossings: CrossingMode::None,
        }
    }
}

#[derive(Deserialize)]
#[serde(try_from = "f32")]
pub(super) struct CornerRadius(pub pautinka::egui::CornerRadius);

impl TryFrom<f32> for CornerRadius {
    type Error = pautinka::egui::PathError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        pautinka::egui::CornerRadius::try_from(value).map(Self)
    }
}

#[derive(Deserialize)]
#[serde(try_from = "f64")]
pub(super) struct Seconds(Duration);

impl Seconds {
    pub fn get(self) -> Duration {
        self.0
    }
}

impl TryFrom<f64> for Seconds {
    type Error = &'static str;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Duration::try_from_secs_f64(value)
            .map(Self)
            .map_err(|_| "Expected finite, non-negative seconds representable as Duration")
    }
}

#[derive(Deserialize)]
#[serde(try_from = "f64")]
pub(super) struct PositiveSeconds(Duration);

impl PositiveSeconds {
    pub fn get(self) -> Duration {
        self.0
    }
}

impl TryFrom<f64> for PositiveSeconds {
    type Error = &'static str;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        let duration = Seconds::try_from(value)?.get();
        if duration.is_zero() {
            return Err("Expected seconds rounding to at least one nanosecond");
        }
        Ok(Self(duration))
    }
}

#[derive(Deserialize)]
#[serde(remote = "RoutingMode", rename_all = "snake_case")]
enum Routing {
    Curved,
    Straight,
    Orthogonal,
    Octilinear,
    Angular15,
}

#[derive(Deserialize)]
#[serde(remote = "Direction", rename_all = "snake_case")]
enum Arrows {
    Undirected,
    Forward,
    Reverse,
    Both,
}

#[derive(Deserialize)]
#[serde(remote = "CrossingMode", rename_all = "snake_case")]
enum Crossings {
    None,
    Gap,
    Bridge,
}
