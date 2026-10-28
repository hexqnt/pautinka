use std::{ops::RangeInclusive, time::Duration};

use crate::{
    scenario::GrowthSettings,
    scene::{Load, NodeCount},
};

pub(super) struct Growth {
    pub enabled: bool,
    pub interval: Duration,
    seconds_range: RangeInclusive<f64>,
    elapsed: Duration,
    maximum: NodeCount,
}

impl Default for Growth {
    fn default() -> Self {
        Self::new(crate::Scenario::default().growth)
    }
}

impl Growth {
    const DEFAULT_SECONDS: RangeInclusive<f64> = 3.0..=30.0;

    pub fn new(settings: GrowthSettings) -> Self {
        let seconds = settings.interval.as_secs_f64();
        Self {
            enabled: settings.enabled,
            interval: settings.interval,
            // Изменение интервала не сужает диапазон: к значению из конфига можно вернуться.
            seconds_range: Self::DEFAULT_SECONDS.start().min(seconds)
                ..=Self::DEFAULT_SECONDS.end().max(seconds),
            elapsed: Duration::ZERO,
            maximum: settings.maximum,
        }
    }

    pub fn maximum(&self) -> usize {
        self.maximum.get()
    }

    pub fn seconds_range(&self) -> RangeInclusive<f64> {
        self.seconds_range.clone()
    }

    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
    }

    pub fn advance(&mut self, step: Duration, load: Load) -> Option<Load> {
        if !self.enabled || load.node_count() >= self.maximum {
            return None;
        }
        let next = load.next_stage()?;
        self.elapsed += step;
        if self.elapsed < self.interval {
            return None;
        }
        self.reset();
        Some(next.with_node_count(next.node_count().min(self.maximum)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_preserves_density_and_advances_at_most_one_stage_per_frame() {
        let mut growth = Growth::default();
        let load = Load::new(700, 5).unwrap();
        assert_eq!(growth.advance(Duration::from_secs(9), load), None);
        assert_eq!(
            growth.advance(Duration::from_secs(1), load),
            Some(Load::new(1_000, 5).unwrap())
        );
        assert_eq!(
            growth.advance(Duration::from_secs(100), load),
            Some(Load::new(1_000, 5).unwrap())
        );
        assert_eq!(
            growth.advance(Duration::from_secs(100), Load::new(20_000, 5).unwrap()),
            None
        );
    }

    #[test]
    fn disabled_growth_and_reset_do_not_consume_the_next_stage() {
        let mut growth = Growth::default();
        growth.advance(Duration::from_secs(7), Load::default());
        growth.enabled = false;
        assert_eq!(
            growth.advance(Duration::from_secs(100), Load::default()),
            None
        );
        assert_eq!(growth.elapsed, Duration::from_secs(7));
        growth.reset();
        growth.enabled = true;
        assert_eq!(
            growth.advance(Duration::from_secs(1), Load::default()),
            None
        );
    }

    #[test]
    fn custom_target_caps_the_last_stage_without_changing_density() {
        let scenario: crate::Scenario = "[run]\nmax_nodes = 750".parse().unwrap();
        let mut growth = Growth::new(scenario.growth);
        let load = Load::new(500, 2.35).unwrap();
        let target = growth.advance(Duration::from_secs(10), load).unwrap();
        assert_eq!(target, Load::new(750, 2.35).unwrap());
        assert_eq!(growth.advance(Duration::from_secs(100), target), None);
    }
}
