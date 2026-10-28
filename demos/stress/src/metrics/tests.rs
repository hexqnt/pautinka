use super::*;

#[test]
fn nearest_rank_percentiles_keep_spikes_and_roll_over() {
    let mut metrics = Metrics::default();
    for milliseconds in [1, 3, 2, 100] {
        metrics.push(Sample {
            interval: Duration::from_millis(milliseconds),
            model: Duration::from_millis(2),
            widget: Duration::from_millis(3),
        });
    }
    metrics.refresh(0.0);
    assert_eq!(metrics.summary.p50_ms, 2.0);
    assert_eq!(metrics.summary.p95_ms, 100.0);
    assert_eq!(metrics.summary.p99_ms, 100.0);
    assert_eq!(metrics.summary.mean_ms, 26.5);
    assert_eq!(metrics.summary.model_ms, 2.0);
    for _ in 0..CAPACITY {
        metrics.push(Sample {
            interval: Duration::from_millis(5),
            ..Sample::default()
        });
    }
    metrics.refresh(0.25);
    assert_eq!(metrics.summary.count, CAPACITY);
    assert_eq!(metrics.summary.max_ms, 5.0);
    metrics.clear();
    metrics.refresh(0.5);
    assert_eq!(metrics.summary.count, 0);
}

#[test]
fn repeated_passes_keep_cpu_cost_and_zero_duration_build_markers() {
    let mut metrics = Metrics::default();
    assert_eq!(metrics.begin_frame(10.0), Duration::ZERO);
    metrics.record_cpu(CpuSample {
        model: Duration::from_millis(1),
        widget: Duration::from_millis(2),
        build: Some(Duration::ZERO),
    });
    assert_eq!(metrics.begin_frame(10.0), Duration::ZERO);
    metrics.record_cpu(CpuSample {
        model: Duration::from_millis(3),
        widget: Duration::from_millis(4),
        build: None,
    });
    metrics.begin_frame(10.02);
    metrics.refresh(10.02);
    assert_eq!(metrics.summary.count, 1);
    assert_eq!(metrics.summary.model_ms, 4.0);
    assert_eq!(metrics.summary.widget_ms, 6.0);
    assert!((metrics.summary.mean_ms - 20.0).abs() < 0.0001);
    assert_eq!(metrics.last_build, Some(Duration::ZERO));
    metrics.record_cpu(CpuSample {
        build: Some(Duration::from_millis(7)),
        ..CpuSample::default()
    });
    metrics.record_cpu(CpuSample {
        build: Some(Duration::from_millis(11)),
        ..CpuSample::default()
    });
    assert_eq!(metrics.last_build, Some(Duration::from_millis(11)));
}

#[test]
fn resume_excludes_time_spent_in_the_background_and_old_samples() {
    let mut metrics = Metrics::default();
    metrics.begin_frame(1.0);
    metrics.record_cpu(CpuSample::default());
    metrics.begin_frame(1.02);
    metrics.resume();
    assert_eq!(metrics.begin_frame(1_000.0), Duration::ZERO);
    metrics.record_cpu(CpuSample::default());
    metrics.begin_frame(1_000.01);
    metrics.refresh(1_000.01);
    assert_eq!(metrics.summary.count, 1);
    assert!((metrics.summary.max_ms - 10.0).abs() < 0.0001);
}
