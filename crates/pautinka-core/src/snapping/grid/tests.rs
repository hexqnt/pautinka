use emath::pos2;

use crate::{SnapIndex, SnapKinds, SnapOptions};

use super::*;

#[test]
fn grid_rejects_invalid_steps_and_captures_whole_intersections_with_radial_tolerance() {
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(SnapGrid::new(invalid).is_none());
    }
    let grid = SnapGrid::new(100.0).unwrap();
    let tolerance = AlignmentSnap::default();
    for (raw, captured) in [
        (pos2(3.0, 4.0), true),
        (pos2(-6.0, 0.0), true),
        (pos2(4.0, 5.0), false),
        (pos2(0.0, 20.0), false),
    ] {
        let result = grid.snap(raw, tolerance, None);
        assert_eq!(result.target, captured.then_some(SnapTarget::Grid));
        assert_eq!(result.position, if captured { Pos2::ZERO } else { raw });
        assert_eq!(result.guides().count(), 0);
    }
    for raw in [pos2(f32::NAN, 0.0), pos2(0.0, f32::INFINITY)] {
        assert_eq!(grid.snap(raw, tolerance, None).target, None);
    }
}

#[test]
fn ties_round_away_from_zero_and_previous_intersections_hold_until_release() {
    let grid = SnapGrid::new(10.0).unwrap();
    for sign in [-1.0, 1.0] {
        let result = grid.snap(
            pos2(5.0, 5.0) * sign,
            AlignmentSnap::new(8.0).unwrap(),
            None,
        );
        assert_eq!(result.position, pos2(10.0, 10.0) * sign);
    }
    let tolerance = AlignmentSnap::default();
    let previous = grid.snap(pos2(4.0, 0.0), tolerance, None);
    for x in [7.0, 9.0] {
        assert_eq!(
            grid.snap(pos2(x, 0.0), tolerance, Some(previous)).position,
            Pos2::ZERO
        );
    }
    assert_eq!(
        grid.snap(pos2(9.01, 0.0), tolerance, Some(previous))
            .position,
        pos2(10.0, 0.0)
    );
    let off_grid = SnapResult {
        position: pos2(3.0, 0.0),
        target: Some(SnapTarget::Grid),
    };
    assert_eq!(
        grid.snap(pos2(4.0, 0.0), tolerance, Some(off_grid))
            .position,
        Pos2::ZERO
    );
}

#[test]
fn extreme_steps_and_coordinates_keep_finite_results() {
    for step in [f64::from_bits(1), 1e-25, 10.0, 1e25, 1e100, f64::MAX] {
        let grid = SnapGrid::new(step).unwrap();
        assert_eq!(grid.step(), step);
        for raw in [
            Pos2::ZERO,
            pos2(1e-25, -1e-25),
            pos2(1e30, -1e30),
            pos2(f32::MAX, f32::MIN),
        ] {
            let result = grid.snap(raw, AlignmentSnap::default(), None);
            assert!(result.position.is_finite());
            let held = grid.snap(raw, AlignmentSnap::default(), Some(result));
            assert_eq!(held, result);
        }
    }
}

#[test]
fn grid_and_guides_choose_nearest_new_target_and_preserve_held_targets() {
    let mut index = SnapIndex::default();
    index.rebuild([pos2(4.0, 100.0)], SnapKinds::VERTICAL);
    let options = SnapOptions {
        kinds: SnapKinds::VERTICAL,
        ..SnapOptions::default()
    };
    let grid = SnapGrid::new(10.0).unwrap();
    let query = |point, previous| index.snap_with_grid(point, options, Some(grid), previous);
    let grid_target = query(pos2(1.0, 0.0), None);
    assert_eq!(grid_target.target, Some(SnapTarget::Grid));
    let guide_target = query(pos2(3.0, 0.0), None);
    assert!(matches!(guide_target.target, Some(SnapTarget::Guide(_))));
    assert!(matches!(
        query(pos2(2.0, 0.0), None).target,
        Some(SnapTarget::Guide(_))
    ));
    assert_eq!(query(pos2(3.0, 0.0), Some(grid_target)), grid_target);
    assert_eq!(query(pos2(1.0, 0.0), Some(guide_target)).position.x, 4.0);
    assert_eq!(
        query(pos2(-6.0, 0.0), Some(guide_target)).position,
        pos2(-10.0, 0.0)
    );
    assert_eq!(
        query(pos2(0.0, 10.0), Some(grid_target)).position,
        pos2(0.0, 10.0)
    );
    let grid_only = SnapOptions {
        kinds: SnapKinds::NONE,
        ..options
    };
    assert_eq!(
        index
            .snap_with_grid(pos2(3.0, 0.0), grid_only, Some(grid), None)
            .target,
        Some(SnapTarget::Grid)
    );
    assert_eq!(
        index
            .snap_with_grid(pos2(1.0, 0.0), grid_only, None, Some(grid_target))
            .target,
        None
    );
}
