use emath::pos2;

use crate::HitPoint;

use super::*;

#[test]
fn close_distinct_positions_do_not_turn_into_loops_in_the_legacy_constructor() {
    for scale in [0.0001, 1.0, 1000.0] {
        let path = EdgePath::new(Pos2::ZERO, pos2(0.01 * scale, 0.0), 0.002 * scale, 0);
        assert!(matches!(path.curve(), Curve::Quadratic(_)));
        assert!(path.bounds().height() < 0.01 * scale);
    }
}

#[test]
fn legacy_loops_preserve_traversal_direction_and_ignore_offset() {
    let position = pos2(10.0, 20.0);
    for (lane, controls) in [
        (
            0,
            [position, pos2(-35.0, -70.0), pos2(55.0, -70.0), position],
        ),
        (
            1,
            [position, pos2(-57.0, -114.0), pos2(77.0, -114.0), position],
        ),
    ] {
        for offset in [0.0, 17.0, f32::NAN, f32::INFINITY] {
            let path = EdgePath::new(position, position, offset, lane);
            assert!(matches!(path.curve(), Curve::Cubic(actual) if actual == controls));
            assert!(path.at_distance(0.0).unwrap().tangent.x < 0.0);
        }
    }
}

#[test]
fn legacy_loop_controls_preserve_float_operations_and_both_endpoints() {
    let bits = |point: Pos2| [point.x.to_bits(), point.y.to_bits()];
    for from in [
        pos2(-0.0, 0.0),
        pos2(0.0, -0.0),
        pos2(f32::from_bits(1), -f32::from_bits(1)),
        pos2(45.0, 90.0),
        pos2(-45.0, 90.0),
        pos2(1e9, -1e9),
        pos2(f32::MAX, -f32::MAX),
    ] {
        let to = Pos2::new(
            if from.x == 0.0 { -from.x } else { from.x },
            if from.y == 0.0 { -from.y } else { from.y },
        );
        for lane in [0, 1, 3, 1023, usize::MAX] {
            let radius = 45.0 + lane as f32 * 22.0;
            let controls = [
                from,
                from + Vec2::new(-radius, -radius * 2.0),
                from + Vec2::new(radius, -radius * 2.0),
                to,
            ];
            let path = EdgePath::new(from, to, f32::NAN, lane);
            assert!(
                matches!(path.curve(), Curve::Cubic(actual) if actual.map(bits) == controls.map(bits))
            );
        }
    }
}

#[test]
fn length_and_arrow_tangents_survive_small_units_and_large_positions() {
    for [a, b] in [
        [Pos2::ZERO, pos2(1e-25, 0.0)],
        [pos2(1e9, 1e9), pos2(1e9 + 1024.0, 1e9)],
    ] {
        for path in [
            EdgePath::try_line(a, b).unwrap(),
            EdgePath::try_quadratic([a, a.lerp(b, 0.5), b]).unwrap(),
        ] {
            assert!(path.length() > 0.0);
            assert!(path.arrow_sample(false).tangent.dot(Vec2::X) > 0.99);
            assert!(path.arrow_sample(true).tangent.dot(-Vec2::X) > 0.99);
        }
    }
}

#[test]
fn hit_tolerances_do_not_underflow_or_overflow_with_world_units() {
    for scale in [1e-25, 0.0001, 1.0, 10000.0, 1e25] {
        let controls = [pos2(0.0, 0.0), pos2(50.0, 80.0), pos2(100.0, 0.0)].map(|p| p * scale);
        for path in [
            EdgePath::try_line(controls[0], controls[2]).unwrap(),
            EdgePath::try_quadratic(controls).unwrap(),
        ] {
            let point = path.midpoint();
            assert!(path.hit_distance_sq(point, scale * 0.1).is_some());
            assert!(
                path.hit_distance_sq(point + Vec2::Y * (100.0 * scale), scale * 0.1)
                    .is_none()
            );
            assert!(path.length() / scale > 99.0);
        }
    }
}

#[test]
fn picking_follows_true_beziers_between_cached_samples_at_all_scales() {
    for scale in [0.0001, 1.0, 1000.0] {
        for controls in [
            [
                pos2(0.0, 0.0),
                pos2(500.0, 1000.0),
                pos2(1000.0, 0.0),
                pos2(1000.0, 0.0),
            ],
            [
                pos2(0.0, 0.0),
                pos2(-500.0, -900.0),
                pos2(700.0, -100.0),
                pos2(50.0, 0.0),
            ],
        ] {
            let controls = controls.map(|p| p * scale);
            for path in [
                EdgePath::try_quadratic(controls[..3].try_into().unwrap()).unwrap(),
                EdgePath::try_cubic(controls).unwrap(),
            ] {
                for step in 0..97 {
                    let t = step as f32 / 96.0;
                    let point = match path.curve() {
                        Curve::Quadratic(c) => bezier::sample(&c, t).position,
                        Curve::Cubic(c) => bezier::sample(&c, t).position,
                        Curve::Polyline(_) => unreachable!(),
                    };
                    let tolerance = scale * 0.002;
                    assert!(path.distance_sq(point).sqrt() < tolerance, "{t} {scale}");
                    assert!(path.spatial_chunks().enumerate().any(|(index, bounds)| {
                        bounds.expand(tolerance).contains(point)
                            && path.chunk_distance_sq(index, point).unwrap().sqrt() < tolerance
                            && path
                                .chunk_hit_distance_sq(index, point, tolerance)
                                .is_some()
                    }));
                }
                let far = pos2(3000.0, 3000.0) * scale;
                let reference = (0..=10000)
                    .map(|i| match path.curve() {
                        Curve::Quadratic(c) => bezier::sample(&c, i as f32 / 10000.0)
                            .position
                            .distance_sq(far),
                        Curve::Cubic(c) => bezier::sample(&c, i as f32 / 10000.0)
                            .position
                            .distance_sq(far),
                        Curve::Polyline(_) => unreachable!(),
                    })
                    .fold(f32::INFINITY, f32::min);
                assert!((path.distance_sq(far) / reference - 1.0).abs() < 0.0001);
            }
        }
    }
}

#[test]
fn inline_lines_preserve_polyline_geometry() {
    let from = pos2(-20.0, 30.0);
    for to in [from, pos2(100.0, -60.0)] {
        let line = EdgePath::try_line(from, to).unwrap();
        let polyline = EdgePath::try_polyline([from, to]).unwrap();
        assert!(matches!(line.data, PathData::Line(_)));
        assert!(matches!(line.curve(), Curve::Polyline(points) if points == [from, to]));
        assert_eq!(line.points(), polyline.points());
        assert_eq!(line.bounds(), polyline.bounds());
        assert_eq!(line.length(), polyline.length());
        assert_eq!(line.midpoint(), polyline.midpoint());
        for reverse in [false, true] {
            assert_eq!(line.arrow(reverse), polyline.arrow(reverse));
        }
        for point in [Pos2::ZERO, from, to, pos2(40.0, 50.0)] {
            assert_eq!(line.distance_sq(point), polyline.distance_sq(point));
            assert_eq!(
                line.chunk_distance_sq(0, point),
                Some(line.distance_sq(point))
            );
        }
        assert_eq!(line.spatial_chunks().collect::<Vec<_>>(), [line.bounds()]);
        assert_eq!(line.chunk_distance_sq(1, from), None);
        assert_eq!(line.chunk_distance_sq(usize::MAX, from), None);
    }
}

#[test]
fn unequal_segments_use_arc_length_for_labels_and_arrows() {
    let path = EdgePath::try_polyline([pos2(0.0, 0.0), pos2(90.0, 0.0), pos2(90.0, 10.0)]).unwrap();
    assert_eq!(path.length(), 100.0);
    assert!(path.midpoint().distance(pos2(50.0, 0.0)) < 0.0001);
    assert_eq!(
        path.at_distance(95.0),
        Some(PathSample {
            position: pos2(90.0, 5.0),
            tangent: Vec2::Y
        })
    );
    assert_eq!(path.distance_sq(pos2(94.0, 5.0)), 16.0);
    assert!(path.arrow(false)[1].distance(pos2(80.0, 0.0)) < 0.0001);
    assert!(path.arrow(true)[0].x > path.arrow(true)[1].x);
    assert_eq!(
        path.bounds(),
        Rect::from_min_max(Pos2::ZERO, pos2(90.0, 10.0))
    );
}

#[test]
fn duplicate_points_and_zero_length_paths_have_defined_samples() {
    let path = EdgePath::try_polyline([Pos2::ZERO, Pos2::ZERO, pos2(10.0, 0.0)]).unwrap();
    assert_eq!(path.at_distance(-1.0).unwrap().position, Pos2::ZERO);
    assert_eq!(path.at_distance(100.0).unwrap().position, pos2(10.0, 0.0));
    let zero = EdgePath::try_line(Pos2::ZERO, Pos2::ZERO).unwrap();
    assert_eq!(zero.length(), 0.0);
    assert_eq!(zero.at_distance(0.0).unwrap().tangent, Vec2::ZERO);
    assert!(zero.at_distance(f32::NAN).is_none());
    assert!(zero.at_distance(f32::INFINITY).is_none());
}

#[test]
fn invalid_external_geometry_is_rejected_at_construction() {
    for coordinate in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let invalid = pos2(coordinate, 0.0);
        for result in [
            EdgePath::try_quadratic([Pos2::ZERO, invalid, Pos2::ZERO]),
            EdgePath::try_cubic([Pos2::ZERO, Pos2::ZERO, invalid, Pos2::ZERO]),
        ] {
            assert_eq!(result.unwrap_err(), PathError::NonFiniteCoordinate);
        }
    }
    assert_eq!(
        EdgePath::try_polyline([]).unwrap_err(),
        PathError::TooFewPoints
    );
    assert_eq!(
        EdgePath::try_polyline([Pos2::ZERO]).unwrap_err(),
        PathError::TooFewPoints
    );
    assert_eq!(
        EdgePath::try_line(Pos2::ZERO, pos2(f32::NAN, 0.0)).unwrap_err(),
        PathError::NonFiniteCoordinate
    );
    assert_eq!(
        EdgePath::try_line(pos2(-f32::MAX, 0.0), pos2(f32::MAX, 0.0)).unwrap_err(),
        PathError::NonFiniteLength
    );
}

#[test]
fn curves_are_bounded_and_arrows_follow_both_directions() {
    for path in [
        EdgePath::new(Pos2::ZERO, pos2(200.0, 0.0), 24.0, 0),
        EdgePath::new(Pos2::ZERO, Pos2::ZERO, 0.0, 0),
    ] {
        for &point in path.points() {
            assert!(path.bounds().contains(point));
        }
        for reverse in [false, true] {
            let [base, tip] = path.arrow(reverse);
            assert!((base.distance(tip) - 1.0).abs() < 0.0001);
            assert!(path.distance_sq(tip) < 0.0001);
        }
    }
}

#[test]
fn arrow_distances_match_cached_directional_samples_for_all_path_kinds() {
    for path in [
        EdgePath::default(),
        EdgePath::try_line(Pos2::ZERO, pos2(100.0, 40.0)).unwrap(),
        EdgePath::new(Pos2::ZERO, pos2(200.0, 0.0), 24.0, 0),
        EdgePath::new(Pos2::ZERO, Pos2::ZERO, 0.0, 0),
        EdgePath::try_polyline([Pos2::ZERO, pos2(90.0, 0.0), pos2(90.0, 10.0)]).unwrap(),
    ] {
        for (reverse, fraction) in [(false, 0.8), (true, 0.2)] {
            assert_eq!(path.arrow_distance(reverse), path.length() * fraction);
            let sample = path.at_distance(path.arrow_distance(reverse)).unwrap();
            let arrow = path.arrow_sample(reverse);
            assert_eq!(arrow.position, sample.position);
            assert_eq!(
                arrow.tangent,
                sample.tangent * if reverse { -1.0 } else { 1.0 }
            );
        }
    }
}

#[test]
fn spatial_parts_cover_curves_between_samples_and_pick_polylines_exactly() {
    for path in [
        EdgePath::try_quadratic([pos2(-10.0, 0.0), pos2(30.0, -50.0), pos2(10.0, 2.0)]).unwrap(),
        EdgePath::try_quadratic([pos2(-100.0, 0.0), pos2(300.0, -500.0), pos2(100.0, 20.0)])
            .unwrap(),
        EdgePath::try_cubic([
            pos2(0.0, 0.0),
            pos2(-500.0, -900.0),
            pos2(700.0, -100.0),
            pos2(50.0, 0.0),
        ])
        .unwrap(),
    ] {
        let parts = path.spatial_chunks().collect::<Vec<_>>();
        for invalid in [parts.len(), usize::MAX] {
            assert_eq!(path.chunk_distance_sq(invalid, Pos2::ZERO), None);
            assert_eq!(path.chunk_hit_distance_sq(invalid, Pos2::ZERO, 1.0), None);
        }
        for step in 0..10001 {
            let t = step as f32 / 10000.0;
            let point = match path.curve() {
                Curve::Quadratic([a, b, c]) => a.lerp(b, t).lerp(b.lerp(c, t), t),
                Curve::Cubic([a, b, c, d]) => a
                    .lerp(b, t)
                    .lerp(b.lerp(c, t), t)
                    .lerp(b.lerp(c, t).lerp(c.lerp(d, t), t), t),
                Curve::Polyline(_) => unreachable!(),
            };
            assert!(
                parts
                    .iter()
                    .any(|bounds| bounds.expand(0.001).contains(point))
            );
        }
    }
    let path = EdgePath::try_polyline([Pos2::ZERO, pos2(0.0, 100.0), pos2(100.0, 100.0)]).unwrap();
    assert_eq!(path.spatial_chunks().len(), 2);
    assert_eq!(path.chunk_distance_sq(0, pos2(3.0, 50.0)), Some(9.0));
    assert_eq!(path.chunk_distance_sq(1, pos2(50.0, 104.0)), Some(16.0));
    assert_eq!(path.chunk_distance_sq(2, Pos2::ZERO), None);
}

#[test]
fn whole_path_hits_match_the_nearest_chunk_with_precise_queries() {
    for scale in [f32::from_bits(8), 1e-25, 1.0, 1000.0, 1e25] {
        for origin in [Pos2::ZERO, pos2(1e9, -1e9)] {
            let [a, b, c, d] = [
                pos2(-20.0, 10.0),
                pos2(100.0, 400.0),
                pos2(700.0, -200.0),
                pos2(1000.0, 10.0),
            ]
            .map(|point| origin + point.to_vec2() * scale);
            for path in [
                EdgePath::default(),
                EdgePath::try_line(a, d).unwrap(),
                EdgePath::try_quadratic([a, b, d]).unwrap(),
                EdgePath::try_cubic([a, b, c, d]).unwrap(),
                EdgePath::try_polyline([a, a, b, c, d, d]).unwrap(),
            ] {
                for point in [a, b, c, d, path.midpoint()] {
                    let precise = HitPoint::new(
                        f64::from(point.x) + f64::from(scale) * 0.25,
                        f64::from(point.y) - f64::from(scale) * 0.5,
                    )
                    .unwrap();
                    for radius in [
                        -scale,
                        -0.0,
                        0.0,
                        scale,
                        scale * 1000.0,
                        f32::MAX,
                        f32::NAN,
                        f32::INFINITY,
                    ] {
                        let expected = (0..path.spatial_chunks().len())
                            .filter_map(|chunk| {
                                path.chunk_hit_distance_sq_precise(chunk, precise, radius)
                            })
                            .min_by(f64::total_cmp);
                        assert_eq!(
                            path.hit_distance_sq_precise(precise, radius)
                                .map(f64::to_bits),
                            expected.map(f64::to_bits),
                            "{scale} {origin:?} {point:?} {radius}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn batched_samples_preserve_independent_traversal_rounding_and_cached_values() {
    // Независимый прежний алгоритм сохраняет эталонную последовательность f32-операций.
    fn reference(path: &EdgePath, mut distance: f32) -> PathSample {
        let mut last = PathSample {
            position: path.points()[0],
            tangent: Vec2::ZERO,
        };
        for (index, pair) in path.points().windows(2).enumerate() {
            let delta = pair[1] - pair[0];
            let length = delta.length();
            if length == 0.0 {
                continue;
            }
            last = PathSample {
                position: pair[1],
                tangent: delta / length,
            };
            if distance <= length {
                if let Some(controls) = path.data.bezier_controls() {
                    let t = (index as f32 + distance / length) / (SAMPLES - 1) as f32;
                    return bezier::sample(controls, t);
                }
                return PathSample {
                    position: pair[0].lerp(pair[1], distance / length),
                    tangent: last.tangent,
                };
            }
            distance -= length;
        }
        last
    }

    fn bits(sample: PathSample) -> [u32; 4] {
        [
            sample.position.x.to_bits(),
            sample.position.y.to_bits(),
            sample.tangent.x.to_bits(),
            sample.tangent.y.to_bits(),
        ]
    }

    fn compare<const N: usize>(path: &EdgePath, distances: [f32; N]) {
        for (distance, sample) in distances.into_iter().zip(path.sample_many(distances)) {
            assert_eq!(bits(sample), bits(reference(path, distance)));
            assert_eq!(bits(path.sample_many([distance])[0]), bits(sample));
            assert_eq!(
                bits(path.at_distance(distance).unwrap()),
                bits(reference(path, distance.clamp(0.0, path.length())))
            );
        }
    }

    for scale in [f32::from_bits(8), 1e-25, 0.0001, 1.0, 1000.0, 1e25] {
        for origin in [pos2(-0.0, 0.0), pos2(1e9, -1e9), pos2(1e30, -1e30)] {
            let [a, b, c, d] = [
                pos2(-0.0, 0.0),
                pos2(50.0, 100.0),
                pos2(-50.0, -20.0),
                pos2(100.0, 20.0),
            ]
            .map(|point| origin + point.to_vec2() * scale);
            for path in [
                EdgePath::try_line(a, d).unwrap(),
                EdgePath::try_quadratic([a, b, d]).unwrap(),
                EdgePath::try_cubic([a, b, c, d]).unwrap(),
                EdgePath::try_polyline([a, a, b, b, c, d, d]).unwrap(),
                EdgePath::try_polyline(
                    (0..SAMPLES)
                        .map(|i| if i % 2 == 0 { a } else { b })
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
                EdgePath::try_polyline(
                    (0..=SAMPLES)
                        .map(|i| if i % 2 == 0 { a } else { b })
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
                EdgePath::try_cubic([a; 4]).unwrap(),
                EdgePath::try_polyline(
                    (0..257)
                        .map(|i| {
                            origin
                                + Vec2::new((i * i % 113) as f32 / 7.0, (i * 17 % 79) as f32 / 13.0)
                                    * scale
                        })
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            ] {
                let length = path.length();
                compare(
                    &path,
                    [
                        length * 0.8,
                        -0.0,
                        length * 0.2,
                        length,
                        length * 1.25,
                        -length,
                        0.0,
                        length * 0.5,
                    ],
                );
                let mut boundary = 0.0;
                for pair in path.points().windows(2) {
                    boundary += pair[0].distance(pair[1]);
                    compare(&path, [boundary.next_up(), boundary, boundary.next_down()]);
                }
                let midpoint = reference(&path, length * 0.5).position;
                assert_eq!(path.midpoint().x.to_bits(), midpoint.x.to_bits());
                assert_eq!(path.midpoint().y.to_bits(), midpoint.y.to_bits());
                for reverse in [false, true] {
                    let mut expected = reference(&path, path.arrow_distance(reverse));
                    expected.tangent *= if reverse { -1.0 } else { 1.0 };
                    assert_eq!(bits(path.arrow_sample(reverse)), bits(expected));
                }
            }
        }
    }
}
