use std::{fmt::Write as _, time::Duration};

use pautinka_test_support::cpu::FrameSample;

fn percentile(values: &[Duration], percentile: usize) -> f64 {
    let index = (values.len() * percentile)
        .div_ceil(100)
        .saturating_sub(1)
        .min(values.len() - 1);
    values[index].as_secs_f64() * 1000.0
}

pub fn row(csv: &mut String, prefix: &str, phase: &str, values: &mut [Duration]) {
    values.sort_unstable();
    let p50 = percentile(values, 50);
    let p95 = percentile(values, 95);
    let p99 = percentile(values, 99);
    let max = values.last().unwrap().as_secs_f64() * 1000.0;
    writeln!(
        csv,
        "{prefix},{phase},{},{p50:.6},{p95:.6},{p99:.6},{max:.6}",
        values.len()
    )
    .unwrap();
}

/// Общие фазы кадра; суммы считаются для каждого кадра до сортировки процентилей.
pub fn frame_rows(csv: &mut String, prefix: &str, ui_phase: &str, samples: &[FrameSample]) {
    let mut values = Vec::with_capacity(samples.len());
    for (phase, name) in [ui_phase, "tessellation", "cpu_total"]
        .into_iter()
        .enumerate()
    {
        values.clear();
        values.extend(samples.iter().map(|sample| {
            [
                sample.ui,
                sample.tessellation,
                sample.ui + sample.tessellation,
            ][phase]
        }));
        row(csv, prefix, name, &mut values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_nearest_rank_and_include_the_tail() {
        let mut samples = [1, 3, 2, 100].map(Duration::from_millis);
        let mut csv = String::new();
        row(&mut csv, "canvas", "ui", &mut samples);
        assert_eq!(
            csv,
            "canvas,ui,4,2.000000,100.000000,100.000000,100.000000\n"
        );
        assert!(samples.is_sorted());
    }

    #[test]
    fn frame_phases_preserve_order_and_sum_before_percentiles() {
        let samples = [(1, 100), (100, 1)].map(|(ui, tessellation)| FrameSample {
            ui: Duration::from_millis(ui),
            tessellation: Duration::from_millis(tessellation),
            shapes: 7,
            vertices: 11,
        });
        let mut csv = String::new();
        frame_rows(&mut csv, "stress", "model_and_ui", &samples);
        assert_eq!(
            csv,
            "stress,model_and_ui,2,1.000000,100.000000,100.000000,100.000000\n\
             stress,tessellation,2,1.000000,100.000000,100.000000,100.000000\n\
             stress,cpu_total,2,101.000000,101.000000,101.000000,101.000000\n"
        );
    }
}
