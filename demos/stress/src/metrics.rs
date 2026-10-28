use std::time::Duration;

pub(crate) const CAPACITY: usize = 600;
const SUMMARY_INTERVAL: f64 = 0.25;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct CpuSample {
    pub model: Duration,
    pub widget: Duration,
    pub build: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default)]
struct Sample {
    interval: Duration,
    model: Duration,
    widget: Duration,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Summary {
    pub count: usize,
    pub mean_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
    pub model_ms: f64,
    pub widget_ms: f64,
}

pub(crate) struct Metrics {
    samples: Box<[Sample; CAPACITY]>,
    next: usize,
    count: usize,
    scratch: [Duration; CAPACITY],
    last_time: Option<f64>,
    pending: Option<CpuSample>,
    next_summary: f64,
    pub last_build: Option<Duration>,
    pub summary: Summary,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            // Единственное выделение при создании: история не занимает стек WASM и не растёт в кадре.
            samples: vec![Sample::default(); CAPACITY]
                .into_boxed_slice()
                .try_into()
                .expect("History must have a fixed capacity"),
            next: 0,
            count: 0,
            scratch: [Duration::ZERO; CAPACITY],
            last_time: None,
            pending: None,
            next_summary: 0.0,
            last_build: None,
            summary: Summary::default(),
        }
    }
}

impl Metrics {
    pub fn clear(&mut self) {
        self.next = 0;
        self.count = 0;
        self.pending = None;
        self.next_summary = 0.0;
        self.summary = Summary::default();
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub fn resume(&mut self) {
        self.last_time = None;
        self.clear();
    }

    pub fn begin_frame(&mut self, now: f64) -> Duration {
        let delta = self
            .last_time
            .replace(now)
            .map_or(0.0, |last| (now - last).max(0.0));
        let interval = Duration::from_secs_f64(delta);
        if !interval.is_zero()
            && let Some(cpu) = self.pending.take()
        {
            // Интервал относится к предыдущему показанному кадру, включая построение новой ступени.
            self.push(Sample {
                interval,
                model: cpu.model,
                widget: cpu.widget,
            });
        }
        interval
    }

    pub fn record_cpu(&mut self, sample: CpuSample) {
        if let Some(build) = sample.build {
            self.last_build = Some(build);
        }
        if let Some(pending) = &mut self.pending {
            // Повторный проход egui имеет тот же timestamp и относится к тому же кадру.
            pending.model += sample.model;
            pending.widget += sample.widget;
        } else {
            self.pending = Some(sample);
        }
    }

    fn push(&mut self, sample: Sample) {
        self.samples[self.next] = sample;
        self.next = (self.next + 1) % CAPACITY;
        self.count = (self.count + 1).min(CAPACITY);
    }

    pub fn refresh(&mut self, now: f64) -> bool {
        if now < self.next_summary {
            return false;
        }
        self.next_summary = now + SUMMARY_INTERVAL;
        if self.count == 0 {
            return true;
        }
        let mut summary = Summary {
            count: self.count,
            ..Summary::default()
        };
        for (sample, value) in self.samples[..self.count].iter().zip(&mut self.scratch) {
            *value = sample.interval;
            summary.mean_ms += sample.interval.as_secs_f64() * 1_000.0;
            summary.model_ms += sample.model.as_secs_f64() * 1_000.0;
            summary.widget_ms += sample.widget.as_secs_f64() * 1_000.0;
        }
        summary.mean_ms /= self.count as f64;
        summary.model_ms /= self.count as f64;
        summary.widget_ms /= self.count as f64;
        let values = &mut self.scratch[..self.count];
        values.sort_unstable();
        let percentile =
            |p: usize| values[(self.count * p).div_ceil(100) - 1].as_secs_f64() * 1_000.0;
        summary.p50_ms = percentile(50);
        summary.p95_ms = percentile(95);
        summary.p99_ms = percentile(99);
        summary.max_ms = values[self.count - 1].as_secs_f64() * 1_000.0;
        self.summary = summary;
        true
    }
}

#[cfg(test)]
mod tests;
