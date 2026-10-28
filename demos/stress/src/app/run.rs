use crate::{scenario::RunLimits, scene::Load};

pub(super) struct Run {
    limits: RunLimits,
    started: Option<f64>,
    at_target: Option<f64>,
}

impl Run {
    pub fn new(limits: RunLimits) -> Self {
        Self {
            limits,
            started: None,
            at_target: None,
        }
    }

    pub fn reset(&mut self) {
        self.started = None;
        self.at_target = None;
    }

    pub fn finished(&mut self, now: f64, load: Load) -> bool {
        // Абсолютное время egui не ускоряет таймер при повторных проходах одного кадра.
        let started = *self.started.get_or_insert(now);
        if self
            .limits
            .duration
            .is_some_and(|duration| now - started >= duration.as_secs_f64())
        {
            return true;
        }
        let Some(target) = self.limits.target else {
            return false;
        };
        if load.node_count() < target.nodes {
            self.at_target = None;
            return false;
        }
        let reached = *self.at_target.get_or_insert(now);
        now - reached >= target.hold.as_secs_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scenario;

    #[test]
    fn duration_starts_at_first_frame_and_restart_resets_it() {
        let scenario: Scenario = "[run]\nduration_secs = 2".parse().unwrap();
        let mut run = Run::new(scenario.limits);
        let load = Load::default();
        assert!(!run.finished(10.0, load));
        assert!(!run.finished(11.0, load));
        assert!(!run.finished(11.0, load));
        assert!(run.finished(12.0, load));
        run.reset();
        assert!(!run.finished(20.0, load));
        assert!(run.finished(22.0, load));
    }

    #[test]
    fn target_is_measured_after_arrival_and_first_limit_wins() {
        let scenario: Scenario = "[run]\nmax_nodes = 750\nhold_secs = 3\nduration_secs = 9"
            .parse()
            .unwrap();
        let mut run = Run::new(scenario.limits);
        let initial = Load::default();
        let target = Load::new(750, 3).unwrap();
        assert!(!run.finished(0.0, initial));
        assert!(!run.finished(4.0, target));
        assert!(!run.finished(6.0, target));
        assert!(run.finished(7.0, target));
        run.reset();
        assert!(!run.finished(0.0, initial));
        assert!(!run.finished(8.0, target));
        assert!(run.finished(9.0, target));
    }

    #[test]
    fn leaving_target_restarts_hold_and_default_run_never_closes() {
        let scenario: Scenario = "[run]\nmax_nodes = 250\nhold_secs = 3".parse().unwrap();
        let mut run = Run::new(scenario.limits);
        let target = Load::new(250, 3).unwrap();
        assert!(!run.finished(0.0, target));
        assert!(!run.finished(2.0, Load::default()));
        assert!(!run.finished(3.0, target));
        assert!(run.finished(6.0, target));
        let mut run = Run::new(RunLimits::default());
        assert!(!run.finished(0.0, target));
        assert!(!run.finished(100_000.0, target));
    }
}
