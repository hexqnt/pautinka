use crate::{Curve, pos2};

use super::*;

const MODES: [RoutingMode; 4] = [
    RoutingMode::Straight,
    RoutingMode::Orthogonal,
    RoutingMode::Octilinear,
    RoutingMode::Angular15,
];

#[test]
fn spacing_and_lane_construction_reject_invalid_inputs() {
    for spacing in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(RibbonSpacing::try_from(spacing).is_err());
    }
    for spacing in [0.0001, 0.49, 5.01, 1000.0] {
        assert_eq!(RibbonSpacing::try_from(spacing).unwrap().get(), spacing);
    }
    assert!(RibbonLane::new(0, 0, RibbonSpacing::default()).is_none());
    assert!(RibbonLane::new(3, 3, RibbonSpacing::default()).is_none());
}

#[test]
fn straight_center_sections_have_requested_spacing_and_reverse_consistently() {
    for mode in MODES {
        let endpoints = RouteEndpoints::Between {
            from: Pos2::ZERO,
            to: pos2(500.0, 0.0),
        };
        for spacing in [0.5, 1.0, 5.0] {
            let spacing = RibbonSpacing::try_from(spacing).unwrap();
            for index in 0..3 {
                let path = mode
                    .route_ribbon(endpoints, RibbonLane::new(index, 3, spacing).unwrap())
                    .unwrap();
                let expected_y = (1.0 - index as f32) * spacing.get();
                assert!(
                    path.points()
                        .windows(2)
                        .any(|pair| (pair[0].y - expected_y).abs() < 0.001
                            && (pair[1].y - expected_y).abs() < 0.001
                            && pair[1].x - pair[0].x > 200.0)
                );
                assert_eq!(path.points()[0], Pos2::ZERO);
                assert_eq!(*path.points().last().unwrap(), pos2(500.0, 0.0));
                let reverse = mode
                    .route_ribbon(
                        RouteEndpoints::Between {
                            from: pos2(500.0, 0.0),
                            to: Pos2::ZERO,
                        },
                        RibbonLane::new(2 - index, 3, spacing).unwrap(),
                    )
                    .unwrap();
                assert_eq!(path.points().len(), reverse.points().len());
                assert!(
                    path.points()
                        .iter()
                        .zip(reverse.points().iter().rev())
                        .all(|(a, b)| a.distance(*b) < 0.001)
                );
            }
        }
    }
}

#[test]
fn constrained_ribbons_keep_allowed_angles_in_all_octants_and_on_short_edges() {
    for mode in MODES {
        for degrees in (0..360).step_by(5) {
            for length in [30.0, 600.0] {
                let from = pos2(12.0, -37.0);
                let to = from + Vec2::angled((degrees as f32).to_radians()) * length;
                for index in 0..5 {
                    let path = mode
                        .route_ribbon(
                            RouteEndpoints::Between { from, to },
                            RibbonLane::new(index, 5, RibbonSpacing::default()).unwrap(),
                        )
                        .unwrap();
                    assert_eq!(path.points()[0], from);
                    assert_eq!(*path.points().last().unwrap(), to);
                    assert!(matches!(path.curve(), Curve::Polyline(_)));
                    let step = match mode {
                        RoutingMode::Orthogonal => 90.0,
                        RoutingMode::Octilinear => 45.0,
                        RoutingMode::Angular15 => 15.0,
                        _ => continue,
                    };
                    for pair in path.points().windows(2) {
                        let delta = pair[1] - pair[0];
                        if delta.length_sq() < 0.000_001 {
                            continue;
                        }
                        let angle = delta.y.atan2(delta.x).to_degrees() / step;
                        assert!(
                            (angle - angle.round()).abs() < 0.002,
                            "{mode:?}, {degrees}, {pair:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn single_edges_coincident_nodes_and_loops_remain_defined() {
    for mode in MODES {
        let single = RibbonLane::new(0, 1, RibbonSpacing::default()).unwrap();
        let endpoints = RouteEndpoints::Between {
            from: Pos2::ZERO,
            to: pos2(300.0, 100.0),
        };
        assert_eq!(
            mode.route_ribbon(endpoints, single).unwrap().points(),
            mode.route(endpoints, 0.0, 0).unwrap().points()
        );
        let coincident = mode
            .route_ribbon(
                RouteEndpoints::Between {
                    from: Pos2::ZERO,
                    to: Pos2::ZERO,
                },
                single,
            )
            .unwrap();
        assert_eq!(coincident.length(), 0.0);
        assert!(
            mode.route_ribbon(
                RouteEndpoints::Loop {
                    position: Pos2::ZERO
                },
                single
            )
            .unwrap()
            .length()
                > 0.0
        );
        assert!(
            mode.route_ribbon(
                RouteEndpoints::Between {
                    from: pos2(-f32::MAX, 0.0),
                    to: pos2(f32::MAX, 0.0)
                },
                RibbonLane::new(0, 2, RibbonSpacing::default()).unwrap()
            )
            .is_err()
        );
    }
}

#[test]
fn tilted_and_bent_routes_keep_perpendicular_spacing_on_the_center_section() {
    let from = Pos2::ZERO;
    let to = pos2(600.0, 240.0);
    for mode in MODES {
        let direction = match mode {
            RoutingMode::Straight => (to - from).normalized(),
            RoutingMode::Orthogonal => Vec2::Y,
            RoutingMode::Octilinear => Vec2::splat(std::f32::consts::FRAC_1_SQRT_2),
            RoutingMode::Angular15 => Vec2::angled(30.0_f32.to_radians()),
            RoutingMode::Curved => unreachable!(),
        };
        for index in 0..3 {
            let path = mode
                .route_ribbon(
                    RouteEndpoints::Between { from, to },
                    RibbonLane::new(index, 3, RibbonSpacing::default()).unwrap(),
                )
                .unwrap();
            let expected = from.lerp(to, 0.5)
                + direction.rot90() * (index as f32 - 1.0) * RibbonSpacing::default().get();
            assert!(
                path.distance_sq(expected) < 0.001,
                "{mode:?}, {index}: {:?}",
                path.points()
            );
        }
    }
}
