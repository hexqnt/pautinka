use emath::pos2;

use super::*;

#[test]
fn axes_snap_independently_and_intersect() {
    let snap = AlignmentSnap::default();
    let horizontal = pos2(100.0, 20.0);
    let vertical = pos2(10.0, 200.0);
    let result = snap.snap(pos2(12.0, 24.0), [horizontal, vertical]);
    assert_eq!(result.position, pos2(10.0, 20.0));
    assert_eq!(result.horizontal, Some(horizontal));
    assert_eq!(result.vertical, Some(vertical));
    let result = snap.snap(pos2(40.0, 24.0), [horizontal]);
    assert_eq!(result.position, pos2(40.0, 20.0));
    assert_eq!(result.vertical, None);
    let result = snap.snap(pos2(12.0, 40.0), [vertical]);
    assert_eq!(result.position, pos2(10.0, 40.0));
    assert_eq!(result.horizontal, None);
}

#[test]
fn nearest_axis_then_nearest_point_wins_independently_of_order() {
    let snap = AlignmentSnap::default();
    let points = [pos2(5.0, 3.0), pos2(2.0, 100.0), pos2(-5.0, -3.0)];
    let result = snap.snap(Pos2::ZERO, points);
    assert_eq!(result.horizontal, Some(points[2]));
    assert_eq!(result.vertical, Some(points[1]));
    assert_eq!(result, snap.snap(Pos2::ZERO, points.into_iter().rev()));
}

#[test]
fn tolerance_boundary_empty_input_and_nonfinite_coordinates() {
    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(AlignmentSnap::new(invalid).is_none());
    }
    let snap = AlignmentSnap::new(2.0).unwrap();
    assert_eq!(snap.tolerance(), 2.0);
    assert_eq!(snap.snap(Pos2::ZERO, []).position, Pos2::ZERO);
    let result = snap.snap(Pos2::ZERO, [pos2(2.0, -2.0)]);
    assert_eq!(result.position, pos2(2.0, -2.0));
    let result = snap.snap(Pos2::ZERO, [pos2(2.01, -2.01), pos2(f32::NAN, 0.0)]);
    assert_eq!(result.horizontal, None);
    assert_eq!(result.vertical, None);
    let result = snap.snap(pos2(f32::INFINITY, 0.0), [Pos2::ZERO]);
    assert_eq!(result.horizontal, None);
    assert_eq!(result.vertical, None);
}

#[test]
fn nearest_source_breaks_axis_ties_and_one_source_can_supply_both_axes() {
    let snap = AlignmentSnap::default();
    let near = pos2(2.0, 3.0);
    let far = pos2(2.0, 5.0);
    for points in [[near, far], [far, near]] {
        let result = snap.snap(Pos2::ZERO, points);
        assert_eq!(result.position, near);
        assert_eq!(result.horizontal, Some(near));
        assert_eq!(result.vertical, Some(near));
    }
}

#[test]
fn extreme_coordinates_and_subnormal_tolerances_keep_nearest_sources() {
    let snap = AlignmentSnap::new(f32::MAX).unwrap();
    let near = pos2(f32::MAX / 2.0, f32::MAX / 2.0);
    let far = pos2(f32::MAX, f32::MAX);
    assert_eq!(snap.snap(Pos2::ZERO, [far, near]).position, near);
    let tiny = f32::from_bits(1);
    let source = pos2(tiny, -tiny);
    let snap = AlignmentSnap::new(tiny).unwrap();
    assert_eq!(snap.snap(Pos2::ZERO, [source]).position, source);
}

#[test]
fn indexed_queries_match_linear_search_on_grids_ties_and_extreme_units() {
    let mut index = AlignmentIndex::default();
    for scale in [f32::from_bits(8), 0.001, 1.0, 1e30] {
        let points: Vec<_> = (0..1000)
            .map(|i| {
                pos2(
                    ((i * 17 % 79) - 39) as f32 * scale,
                    ((i * 31 % 83) - 41) as f32 * scale,
                )
            })
            .chain([
                pos2(-0.0, 10.0 * scale),
                pos2(0.0, -10.0 * scale),
                pos2(f32::NAN, 0.0),
            ])
            .collect();
        let snap = AlignmentSnap::new(6.0 * scale).unwrap();
        index.rebuild(points.iter().copied());
        for i in 0..2000 {
            let position = pos2(
                ((i * 19 % 149) - 74) as f32 * scale,
                ((i * 37 % 151) - 75) as f32 * scale,
            );
            assert_eq!(
                index.snap(position, snap, None),
                snap.snap(position, points.iter().copied())
            );
        }
    }
    index.rebuild([]);
    assert_eq!(
        index
            .snap(Pos2::ZERO, AlignmentSnap::default(), None)
            .vertical,
        None
    );
}

#[test]
fn hysteresis_holds_axes_independently_until_release_and_drops_removed_axes() {
    let policy = AlignmentSnap::default();
    let mut index = AlignmentIndex::default();
    index.rebuild([pos2(0.0, 100.0), pos2(8.0, 200.0), pos2(200.0, 0.0)]);
    let first = index.snap(pos2(1.0, 1.0), policy, None);
    assert_eq!(first.position, Pos2::ZERO);
    let held = index.snap(pos2(9.0, 7.0), policy, Some(first));
    assert_eq!(held.position, Pos2::ZERO);
    let switched = index.snap(pos2(9.1, 7.0), policy, Some(held));
    assert_eq!(switched.position, pos2(8.0, 0.0));
    let released = index.snap(pos2(30.0, 30.0), policy, Some(switched));
    assert_eq!(released.position, pos2(30.0, 30.0));
    assert_eq!(released.horizontal, None);
    assert_eq!(released.vertical, None);
    index.rebuild([pos2(8.0, 200.0)]);
    let removed = index.snap(pos2(7.0, 7.0), policy, Some(first));
    assert_eq!(removed.position, pos2(8.0, 7.0));
    assert_eq!(removed.horizontal, None);
}

#[test]
fn release_threshold_and_scaling_preserve_policy_invariants() {
    let policy = AlignmentSnap::new(4.0).unwrap();
    assert_eq!(policy.release_tolerance(), 6.0);
    for invalid in [3.9, f32::NAN, f32::INFINITY] {
        assert!(policy.with_release_tolerance(invalid).is_none());
    }
    let policy = policy.with_release_tolerance(8.0).unwrap();
    let scaled = policy.scaled(0.25).unwrap();
    assert_eq!(scaled.tolerance(), 1.0);
    assert_eq!(scaled.release_tolerance(), 2.0);
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(policy.scaled(invalid).is_none());
    }
    for scale in [f64::MIN_POSITIVE, f64::MAX] {
        let scaled = policy.scaled(scale).unwrap();
        assert!(scaled.tolerance() > 0.0);
        assert!(scaled.release_tolerance().is_finite());
        assert!(scaled.release_tolerance() >= scaled.tolerance());
    }
}

#[test]
fn duplicate_sources_and_signed_zeros_preserve_nearest_axis_order() {
    let points = [
        pos2(-0.0, 10.0),
        pos2(0.0, -10.0),
        pos2(-0.0, -0.0),
        pos2(0.0, 0.0),
    ];
    let mut index = AlignmentIndex::default();
    let policy = AlignmentSnap::default();
    for sources in [points, [points[3], points[2], points[1], points[0]]] {
        index.rebuild(sources.into_iter().cycle().take(1000));
        for query in [pos2(1.0, 3.0), pos2(-1.0, -8.0), pos2(0.0, 5.0)] {
            assert_eq!(index.snap(query, policy, None), policy.snap(query, points));
        }
    }
}
