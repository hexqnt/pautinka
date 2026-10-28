mod raw;

use std::{str::FromStr, time::Duration};

use pautinka::egui::CrossingMode;

use crate::scene::{Appearance, Load, Motion, NodeCount, SceneStyle, Topology};

/// Разобранный сценарий: диапазоны и согласованность параметров проверяются до создания сцены.
#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(try_from = "raw::Scenario")]
pub struct Scenario {
    pub(crate) seed: u32,
    pub(crate) load: Load,
    pub(crate) style: SceneStyle,
    pub(crate) topology: Topology,
    pub(crate) motion: Motion,
    pub(crate) animate: bool,
    pub(crate) growth: GrowthSettings,
    pub(crate) limits: RunLimits,
    pub(crate) view: ViewSettings,
    pub(crate) pattern: Pattern,
    pub(crate) crossings: CrossingMode,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct GrowthSettings {
    pub enabled: bool,
    pub interval: Duration,
    pub maximum: NodeCount,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RunLimits {
    pub duration: Option<Duration>,
    pub target: Option<Target>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Target {
    pub nodes: NodeCount,
    pub hold: Duration,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Pattern {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ViewSettings {
    pub auto_fit: bool,
    pub labels: bool,
    pub lod: bool,
    pub minimap: bool,
    pub grid: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            auto_fit: true,
            labels: false,
            lod: true,
            minimap: true,
            grid: true,
        }
    }
}

impl Default for Scenario {
    fn default() -> Self {
        raw::Scenario::default()
            .try_into()
            .expect("Default scenario must be valid")
    }
}

impl FromStr for Scenario {
    type Err = toml::de::Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        toml::from_str(source)
    }
}

impl TryFrom<raw::Scenario> for Scenario {
    type Error = &'static str;

    fn try_from(raw: raw::Scenario) -> Result<Self, Self::Error> {
        let load = Load::new(raw.load.nodes, raw.load.edges_per_node)?;
        let topology = Topology::new(
            raw.topology.locality,
            raw.topology.local_radius,
            raw.topology.hub_share,
            raw.topology.hubs,
            raw.topology.backbone,
        )?;
        let motion = Motion::new(raw.motion.fraction, raw.motion.speed)?;
        let appearance = Appearance::new(raw.nodes.kind, raw.nodes.colors, raw.nodes.radius)?;
        if raw.run.hold_secs.is_some() && raw.run.max_nodes.is_none() {
            return Err("run.hold_secs requires run.max_nodes");
        }
        let target = raw.run.max_nodes.map(|nodes| Target {
            nodes,
            hold: raw
                .run
                .hold_secs
                .map_or(Duration::from_secs(10), raw::Seconds::get),
        });
        if let Some(target) = target {
            if target.nodes < load.node_count() {
                return Err("run.max_nodes must be at least load.nodes");
            }
            if !raw.growth.enabled && target.nodes > load.node_count() {
                return Err("Reaching run.max_nodes requires growth.enabled = true");
            }
        }
        Ok(Self {
            seed: raw.seed,
            load,
            style: SceneStyle {
                appearance,
                routing: raw.edges.routing,
                corner_radius: raw.edges.corner_radius.0,
                direction: raw.edges.direction,
            },
            topology,
            motion,
            animate: raw.motion.animate,
            growth: GrowthSettings {
                enabled: raw.growth.enabled,
                interval: raw.growth.interval_secs.get(),
                maximum: target.map_or(NodeCount::MAX, |target| target.nodes),
            },
            limits: RunLimits {
                duration: raw.run.duration_secs.map(raw::PositiveSeconds::get),
                target,
            },
            view: raw.view,
            pattern: raw.edges.pattern,
            crossings: raw.edges.crossings,
        })
    }
}

#[cfg(test)]
mod tests;
