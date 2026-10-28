use emath::pos2;

use super::*;

const MODES: [RoutingMode; 5] = [
    RoutingMode::Curved,
    RoutingMode::Straight,
    RoutingMode::Orthogonal,
    RoutingMode::Octilinear,
    RoutingMode::Angular15,
];
const ANGULAR_MODES: [RoutingMode; 3] = [
    RoutingMode::Orthogonal,
    RoutingMode::Octilinear,
    RoutingMode::Angular15,
];

#[test]
fn oriented_anchors_preserve_port_positions_and_custom_loops_respect_scale_and_direction() {
    for scale in [0.0001, 1.0, 10000.0] {
        let from = Anchor::new(Pos2::ZERO, Vec2::X).unwrap();
        let to = Anchor::new(Pos2::new(100.0 * scale, 40.0 * scale), -Vec2::X).unwrap();
        for mode in MODES {
            let path = mode.route_anchors(from, to, 10.0 * scale, 0.0).unwrap();
            assert_eq!(path.points()[0], from.position());
            assert_eq!(*path.points().last().unwrap(), to.position());
            if mode == RoutingMode::Curved {
                assert!(path.at_distance(0.0).unwrap().tangent.dot(Vec2::X) > 0.99);
            }
            let loops = LoopOptions::new(10.0 * scale, 5.0 * scale, Vec2::X).unwrap();
            let path = mode
                .route_with(
                    RouteEndpoints::Loop {
                        position: Pos2::ZERO,
                    },
                    0.0,
                    1,
                    loops,
                )
                .unwrap();
            assert!(path.bounds().max.x > 20.0 * scale);
            assert!(path.bounds().min.x >= 0.0);
        }
    }
    assert!(LoopOptions::new(0.0, 1.0, Vec2::X).is_err());
    assert!(LoopOptions::new(1.0, 1.0, Vec2::ZERO).is_err());
}
#[test]
fn compacted_lines_preserve_all_owned_polyline_queries() {
    let from = pos2(-20.0, 30.0);
    for to in [from, pos2(100.0, -60.0)] {
        let path = polyline([from, from, from.lerp(to, 0.5), to, to]).unwrap();
        let owned = EdgePath::try_polyline([from, to]).unwrap();
        assert!(matches!(path.curve(), crate::Curve::Polyline(points) if points == owned.points()));
        assert_eq!(path.points(), owned.points());
        assert_eq!(path.bounds(), owned.bounds());
        assert_eq!(path.length(), owned.length());
        assert_eq!(path.midpoint(), owned.midpoint());
        assert_eq!(
            path.spatial_chunks().collect::<Vec<_>>(),
            owned.spatial_chunks().collect::<Vec<_>>()
        );
        for reverse in [false, true] {
            assert_eq!(path.arrow(reverse), owned.arrow(reverse));
            assert_eq!(path.arrow_sample(reverse), owned.arrow_sample(reverse));
        }
        for distance in [-1.0, 0.0, 5.0, 1000.0, f32::NAN, f32::INFINITY] {
            assert_eq!(path.at_distance(distance), owned.at_distance(distance));
        }
        for point in [from, to, pos2(40.0, 50.0)] {
            let precise =
                crate::HitPoint::new(f64::from(point.x) + 0.25, f64::from(point.y)).unwrap();
            assert_eq!(path.distance_sq(point), owned.distance_sq(point));
            for radius in [-1.0, 0.0, 10.0, 100.0, f32::NAN, f32::INFINITY] {
                assert_eq!(
                    path.hit_distance_sq(point, radius),
                    owned.hit_distance_sq(point, radius)
                );
                assert_eq!(
                    path.hit_distance_sq_precise(precise, radius),
                    owned.hit_distance_sq_precise(precise, radius)
                );
                for chunk in [0, 1, usize::MAX] {
                    assert_eq!(
                        path.chunk_distance_sq(chunk, point),
                        owned.chunk_distance_sq(chunk, point)
                    );
                    assert_eq!(
                        path.chunk_hit_distance_sq(chunk, point, radius),
                        owned.chunk_hit_distance_sq(chunk, point, radius)
                    );
                    assert_eq!(
                        path.chunk_hit_distance_sq_precise(chunk, precise, radius),
                        owned.chunk_hit_distance_sq_precise(chunk, precise, radius)
                    );
                }
            }
        }
    }
    assert_eq!(polyline([]).unwrap_err(), PathError::TooFewPoints);
    assert_eq!(polyline([from]).unwrap().points(), [from, from]);
}

fn check_segments(path: &EdgePath, mode: RoutingMode) {
    for pair in path.points().windows(2) {
        let delta = pair[1] - pair[0];
        let axis = delta.x == 0.0 || delta.y == 0.0;
        let diagonal = (delta.x.abs() - delta.y.abs()).abs() < 0.001;
        let angular15 = if delta.length_sq() < 0.000_001 {
            true
        } else {
            let steps = delta.y.atan2(delta.x).to_degrees() / 15.0;
            (steps - steps.round()).abs() < 0.001
        };
        assert!(
            axis || (mode == RoutingMode::Octilinear && diagonal)
                || (mode == RoutingMode::Angular15 && angular15),
            "{pair:?}"
        );
        assert!(path.bounds().contains(pair[0]));
        assert!(path.distance_sq(pair[0].lerp(pair[1], 0.5)) < 0.000_001);
    }
}

#[test]
fn routes_preserve_endpoints_and_angles_in_all_octants() {
    let from = pos2(12.5, -37.0);
    for mode in ANGULAR_MODES {
        for delta in [
            Vec2::new(100.0, 60.0),
            Vec2::new(60.0, 100.0),
            Vec2::new(100.0, 0.0),
            Vec2::new(0.0, 100.0),
            Vec2::splat(80.0),
            Vec2::ZERO,
        ] {
            for sign in [
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(-1.0, -1.0),
            ] {
                let to = from + delta * sign;
                for offset in [-24.0, 0.0, 24.0] {
                    let path = mode
                        .route(RouteEndpoints::Between { from, to }, offset, 0)
                        .unwrap();
                    assert_eq!(path.points().first(), Some(&from));
                    assert_eq!(path.points().last(), Some(&to));
                    check_segments(&path, mode);
                }
            }
        }
    }
}

#[test]
fn loops_are_explicit_and_lanes_have_distinct_paths() {
    let position = pos2(10.0, 20.0);
    for mode in MODES {
        let coincident = mode
            .route(
                RouteEndpoints::Between {
                    from: position,
                    to: position,
                },
                0.0,
                0,
            )
            .unwrap();
        assert_eq!(coincident.length(), 0.0);
        let first = mode
            .route(RouteEndpoints::Loop { position }, 0.0, 0)
            .unwrap();
        let second = mode
            .route(RouteEndpoints::Loop { position }, 0.0, 1)
            .unwrap();
        assert!(second.length() > first.length());
        assert_eq!(first.points().first(), first.points().last());
        if !matches!(mode, RoutingMode::Curved | RoutingMode::Straight) {
            check_segments(&first, mode);
        }
    }
    for mode in ANGULAR_MODES {
        let endpoints = RouteEndpoints::Between {
            from: position,
            to: position + Vec2::new(100.0, 0.0),
        };
        let a = mode.route(endpoints, -24.0, 0).unwrap();
        let b = mode.route(endpoints, 24.0, 1).unwrap();
        assert_ne!(a.midpoint(), b.midpoint());
    }
}

#[test]
fn angular15_covers_all_sectors_and_uses_15_and_30_degree_segments() {
    let from = pos2(12.5, -37.0);
    for degrees in 0..360 {
        let angle = (degrees as f32).to_radians();
        let to = from + Vec2::angled(angle) * 240.0;
        for offset in [-24.0, 0.0, 24.0] {
            let path = RoutingMode::Angular15
                .route(RouteEndpoints::Between { from, to }, offset, 0)
                .unwrap();
            assert_eq!(path.points().first(), Some(&from));
            assert_eq!(path.points().last(), Some(&to));
            check_segments(&path, RoutingMode::Angular15);
        }
    }
    for (rise, expected_angle) in [(20.0, 15.0), (40.0, 30.0)] {
        let path = RoutingMode::Angular15
            .route(
                RouteEndpoints::Between {
                    from: Pos2::ZERO,
                    to: pos2(100.0, rise),
                },
                0.0,
                0,
            )
            .unwrap();
        assert!(path.points().windows(2).any(|pair| {
            let delta = pair[1] - pair[0];
            (delta.y.atan2(delta.x).to_degrees() - expected_angle).abs() < 0.001
        }));
    }
}

#[test]
fn straight_routes_connect_centers_without_bends_or_lane_offsets() {
    let from = pos2(12.5, -37.0);
    for to in [from, pos2(-70.0, 18.0), pos2(120.0, 85.0)] {
        for offset in [-24.0, 0.0, 24.0] {
            let path = RoutingMode::Straight
                .route(RouteEndpoints::Between { from, to }, offset, 3)
                .unwrap();
            assert_eq!(path.points(), [from, to]);
            assert!((path.length() - from.distance(to)).abs() < 0.001);
        }
    }
}

#[test]
fn rejects_invalid_input() {
    for mode in MODES {
        for endpoints in [
            RouteEndpoints::Between {
                from: Pos2::ZERO,
                to: pos2(f32::NAN, 0.0),
            },
            RouteEndpoints::Loop {
                position: pos2(f32::INFINITY, 0.0),
            },
        ] {
            assert!(mode.route(endpoints, 0.0, 0).is_err());
        }
        assert!(
            mode.route(
                RouteEndpoints::Loop {
                    position: Pos2::ZERO
                },
                f32::NAN,
                0
            )
            .is_err()
        );
        assert!(
            mode.route(
                RouteEndpoints::Between {
                    from: pos2(-f32::MAX, 0.0),
                    to: pos2(f32::MAX, 0.0)
                },
                0.0,
                0
            )
            .is_err()
        );
    }
}
