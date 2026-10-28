use emath::pos2;

use super::*;

fn prepared(points: impl IntoIterator<Item = Pos2>, kinds: SnapKinds) -> (SnapIndex, SnapOptions) {
    let mut index = SnapIndex::default();
    index.rebuild(points, kinds);
    (
        index,
        SnapOptions {
            kinds,
            ..SnapOptions::default()
        },
    )
}

#[test]
fn kinds_are_independent_and_empty_modes_do_not_snap() {
    let mut kinds = SnapKinds::HORIZONTAL | SnapKinds::DIAGONAL_UP;
    assert!(kinds.contains(SnapKinds::HORIZONTAL));
    assert!(!kinds.contains(SnapKinds::VERTICAL));
    kinds.set(SnapKinds::HORIZONTAL, false);
    assert_eq!(kinds, SnapKinds::DIAGONAL_UP);
    kinds |= SnapKinds::DIAGONAL_DOWN;
    assert_eq!(kinds, SnapKinds::DIAGONALS);
    kinds.set(SnapKinds::ALL, false);
    assert!(kinds.is_empty());
    let (index, options) = prepared([Pos2::ZERO], kinds);
    let result = index.snap(pos2(1.0, 2.0), options, None);
    assert_eq!(result.position, pos2(1.0, 2.0));
    assert_eq!(result.target, None);
}

#[test]
fn diagonal_projection_uses_perpendicular_distance() {
    for (kind, point, expected) in [
        (SnapKind::DiagonalUp, pos2(40.0, -37.0), pos2(38.5, -38.5)),
        (SnapKind::DiagonalDown, pos2(40.0, 37.0), pos2(38.5, 38.5)),
    ] {
        let (index, options) = prepared([Pos2::ZERO], kind.flag());
        let result = index.snap(point, options, None);
        assert_eq!(result.position, expected);
        assert_eq!(
            result.target,
            Some(SnapTarget::Guide(SnapGuide {
                kind,
                source: Pos2::ZERO
            }))
        );
    }
    let (index, options) = prepared([Pos2::ZERO], SnapKinds::DIAGONAL_UP);
    assert!(index.snap(pos2(0.0, 8.4), options, None).target.is_some());
    assert!(index.snap(pos2(0.0, 8.6), options, None).target.is_none());
}

#[test]
fn mixed_and_diagonal_intersections_keep_their_sources() {
    for (points, kinds, pointer, expected) in [
        (
            [pos2(0.0, 10.0), pos2(20.0, 0.0)],
            SnapKinds::DIAGONALS,
            pos2(14.0, -4.0),
            pos2(15.0, -5.0),
        ),
        (
            [pos2(100.0, 10.0), Pos2::ZERO],
            SnapKinds::HORIZONTAL | SnapKinds::DIAGONAL_DOWN,
            pos2(11.0, 9.0),
            pos2(10.0, 10.0),
        ),
    ] {
        let (index, options) = prepared(points, kinds);
        let result = index.snap(pointer, options, None);
        assert_eq!(result.position, expected);
        let Some(SnapTarget::Intersection(guides)) = result.target else {
            panic!("expected intersection")
        };
        assert_ne!(guides[0].source, guides[1].source);
        for guide in guides {
            assert_eq!(
                guide.kind.coordinate(result.position),
                guide.kind.coordinate(guide.source)
            );
        }
    }
}

#[test]
fn distant_mixed_intersections_are_rejected_in_favor_of_a_nearby_guide() {
    let (index, options) = prepared(
        [pos2(-100.0, 6.0), pos2(-100.0, -108.0)],
        SnapKinds::HORIZONTAL | SnapKinds::DIAGONAL_DOWN,
    );
    let result = index.snap(Pos2::ZERO, options, None);
    assert_eq!(result.position, pos2(4.0, -4.0));
    assert!(matches!(
        result.target,
        Some(SnapTarget::Guide(SnapGuide {
            kind: SnapKind::DiagonalDown,
            ..
        }))
    ));
}

#[test]
fn diagonal_hysteresis_releases_removed_and_disabled_guides() {
    let (mut index, mut options) = prepared(
        [pos2(-100.0, 100.0), pos2(-100.0, 108.0)],
        SnapKinds::DIAGONAL_UP,
    );
    let first = index.snap(pos2(0.0, 1.0), options, None);
    let held = index.snap(pos2(4.0, 5.0), options, Some(first));
    assert_eq!(held.position, pos2(-0.5, 0.5));
    let released = index.snap(pos2(7.0, 7.0), options, Some(held));
    assert_eq!(released.position, pos2(4.0, 4.0));
    index.rebuild([pos2(-100.0, 108.0)], options.kinds);
    assert_eq!(
        index.snap(pos2(4.0, 5.0), options, Some(first)).position,
        pos2(3.5, 4.5)
    );
    options.kinds = SnapKinds::NONE;
    assert_eq!(
        index.snap(pos2(4.0, 5.0), options, Some(first)).target,
        None
    );
}

#[test]
fn indexed_sources_match_exhaustive_search_for_each_family() {
    let points: Vec<_> = (0..400)
        .map(|i| pos2(((i * 17 % 79) - 39) as f32, ((i * 31 % 83) - 41) as f32))
        .chain([pos2(f32::NAN, 0.0), Pos2::ZERO, pos2(-0.0, -0.0)])
        .collect();
    for kind in SnapKind::ALL {
        let (index, options) = prepared(points.iter().copied(), kind.flag());
        for i in 0..600 {
            let position = pos2(((i * 19 % 149) - 74) as f32, ((i * 37 % 151) - 75) as f32);
            let expected = points
                .iter()
                .copied()
                .filter(|point| point.is_finite())
                .filter(|point| {
                    kind.distance(position, *point) <= f64::from(options.tolerance.tolerance())
                })
                .min_by(|a, b| {
                    kind.distance(position, *a)
                        .total_cmp(&kind.distance(position, *b))
                        .then_with(|| {
                            (kind.along(*a) - kind.along(position))
                                .abs()
                                .total_cmp(&(kind.along(*b) - kind.along(position)).abs())
                        })
                        .then_with(|| a.x.total_cmp(&b.x))
                        .then_with(|| a.y.total_cmp(&b.y))
                });
            assert_eq!(
                index
                    .snap(position, options, None)
                    .guides()
                    .next()
                    .map(|guide| guide.source),
                expected
            );
        }
    }
}

#[test]
fn extreme_positions_and_nonfinite_input_do_not_create_invalid_results() {
    let (index, mut options) = prepared([pos2(f32::MAX, f32::MAX), Pos2::ZERO], SnapKinds::ALL);
    options.tolerance = AlignmentSnap::new(f32::MAX).unwrap();
    for position in [
        pos2(f32::MAX, 0.0),
        pos2(0.0, f32::MAX),
        pos2(-f32::MAX, f32::MAX),
    ] {
        assert!(index.snap(position, options, None).position.is_finite());
    }
    assert_eq!(
        index.snap(pos2(f32::INFINITY, 0.0), options, None).target,
        None
    );
}

#[test]
fn rebuilding_modes_releases_unprepared_families_and_reuses_sources() {
    let points = [Pos2::ZERO, pos2(-0.0, -0.0), pos2(20.0, 40.0)];
    let options = SnapOptions {
        kinds: SnapKinds::ALL,
        ..SnapOptions::default()
    };
    let mut index = SnapIndex::default();
    index.rebuild(points, options.kinds);
    let mut previous = Some(index.snap(pos2(1.0, 2.0), options, None));
    for kinds in (0..=SnapKinds::ALL.0).map(SnapKinds) {
        index.rebuild(points.into_iter().rev(), kinds);
        for position in [Pos2::ZERO, pos2(2.0, 3.0), pos2(-4.0, -5.0)] {
            let result = index.snap(position, options, previous);
            assert_eq!(
                result,
                index.snap(position, SnapOptions { kinds, ..options }, previous)
            );
            assert!(
                result
                    .guides()
                    .all(|guide| kinds.contains(guide.kind.flag()))
            );
            previous = Some(result);
        }
    }
}

#[test]
fn diagonal_coordinate_collisions_preserve_exact_sources_for_reordered_input() {
    let query = pos2(1e30, 0.0);
    let first = pos2(1e30, 2e-20);
    let nearest = pos2(1e30, 1e-20);
    let mut index = SnapIndex::default();
    for kind in [SnapKind::DiagonalUp, SnapKind::DiagonalDown] {
        let options = SnapOptions {
            kinds: kind.flag(),
            ..SnapOptions::default()
        };
        for kinds in [kind.flag(), SnapKinds::ALL] {
            for points in [[first, nearest], [nearest, first]] {
                index.rebuild(points.into_iter().cycle().take(41), kinds);
                let result = index.snap(query, options, None);
                assert!(matches!(result.target, Some(SnapTarget::Guide(_))));
                assert_eq!(result.position.x.to_bits(), query.x.to_bits());
                assert_eq!(result.position.y.to_bits(), query.y.to_bits());
                assert!(
                    result
                        .guides()
                        .map(|guide| (
                            guide.kind,
                            guide.source.x.to_bits(),
                            guide.source.y.to_bits(),
                        ))
                        .eq([(kind, nearest.x.to_bits(), nearest.y.to_bits())])
                );
            }
        }
    }

    let mut points = vec![first, nearest];
    for i in 2..21 {
        let x = ((i * 17 % 79) as f32 - 39.0) * 1e30;
        let y = ((i * 31 % 83) as f32 - 41.0) * 1e30;
        points.push(match i % 4 {
            0 => pos2(x, y),
            1 => pos2(x, ((i * 13 % 11) as f32 - 5.0) * 1e-20),
            2 => pos2(((i * 7 % 13) as f32 - 6.0) * 1e-20, y),
            _ => pos2(1e30, ((i * 19 % 17) as f32 - 8.0) * 1e-20),
        });
    }
    let options = SnapOptions {
        kinds: SnapKinds::DIAGONAL_UP,
        ..SnapOptions::default()
    };
    for kinds in [
        SnapKinds::ALL,
        SnapKinds::HORIZONTAL | SnapKinds::DIAGONAL_UP,
    ] {
        for _ in 0..points.len() {
            index.rebuild(points.iter().copied(), kinds);
            let result = index.snap(query, options, None);
            assert!(matches!(result.target, Some(SnapTarget::Guide(_))));
            assert!(
                result
                    .guides()
                    .map(|guide| (
                        guide.kind,
                        guide.source.x.to_bits(),
                        guide.source.y.to_bits(),
                    ))
                    .eq([(
                        SnapKind::DiagonalUp,
                        query.x.to_bits(),
                        (-4e-20_f32).to_bits()
                    )])
            );
            points.rotate_left(1);
        }
    }
}
