use crate::{Curve, RouteEndpoints, RoutingMode, pos2};

use super::*;

#[test]
fn radius_accepts_only_finite_nonnegative_values() {
    for value in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            CornerRadius::try_from(value),
            Err(PathError::InvalidCornerRadius)
        );
    }
    assert_eq!(CornerRadius::default().get(), 0.0);
    assert_eq!(CornerRadius::try_from(f32::MAX).unwrap().get(), f32::MAX);
}

#[test]
fn right_angle_has_requested_radius_and_preserves_endpoints_in_both_directions() {
    for sign in [-1.0, 1.0] {
        let points = [pos2(0.0, 0.0), pos2(100.0, 0.0), pos2(100.0, sign * 100.0)];
        let path = EdgePath::try_polyline(points)
            .unwrap()
            .with_corner_radius(CornerRadius::try_from(20.0).unwrap())
            .unwrap();
        assert_eq!(path.points().first(), points.first());
        assert_eq!(path.points().last(), points.last());
        assert_eq!(path.points()[1], pos2(80.0, 0.0));
        assert_eq!(
            path.points()[path.points().len() - 2],
            pos2(100.0, sign * 20.0)
        );
        let center = pos2(80.0, sign * 20.0);
        for point in &path.points()[1..path.points().len() - 1] {
            assert!((point.distance(center) - 20.0).abs() < 0.000_01);
        }
        assert!(path.distance_sq(pos2(100.0, 0.0)) > 60.0);
        assert!((path.length() - (160.0 + std::f32::consts::PI * 10.0)).abs() < 0.02);
    }
}

#[test]
fn short_segments_limit_radius_and_degenerate_vertices_stay_finite() {
    let points = [
        pos2(0.0, 0.0),
        pos2(0.0, 0.0),
        pos2(10.0, 0.0),
        pos2(10.0, 2.0),
        pos2(20.0, 2.0),
        pos2(10.0, 2.0),
    ];
    let path = EdgePath::try_polyline(points)
        .unwrap()
        .with_corner_radius(CornerRadius::try_from(f32::MAX).unwrap())
        .unwrap();
    assert_eq!(path.points().first(), points.first());
    assert_eq!(path.points().last(), points.last());
    assert!(path.points().iter().all(|point| point.is_finite()));
    assert!(path.points().contains(&pos2(9.0, 0.0)));
    assert!(path.points().contains(&pos2(10.0, 1.0)));
    assert!(path.length().is_finite());
}

#[test]
fn all_angular_modes_round_routes_while_zero_radius_and_curves_are_unchanged() {
    for mode in [
        RoutingMode::Orthogonal,
        RoutingMode::Octilinear,
        RoutingMode::Angular15,
        RoutingMode::Straight,
        RoutingMode::Curved,
    ] {
        let endpoints = RouteEndpoints::Between {
            from: pos2(0.0, 0.0),
            to: pos2(200.0, 40.0),
        };
        let sharp = mode.route(endpoints, 0.0, 0).unwrap();
        let zero = sharp
            .clone()
            .with_corner_radius(CornerRadius::default())
            .unwrap();
        assert_eq!(zero.points(), sharp.points());
        let rounded = sharp
            .clone()
            .with_corner_radius(CornerRadius::try_from(10.0).unwrap())
            .unwrap();
        if matches!(mode, RoutingMode::Straight | RoutingMode::Curved) {
            assert_eq!(rounded.points(), sharp.points());
        } else {
            assert!(rounded.points().len() > sharp.points().len());
            assert!(rounded.length() < sharp.length());
        }
    }
}

#[test]
fn touching_arcs_have_no_zero_length_segments() {
    let points = [
        pos2(0.0, 0.0),
        pos2(10.0, 0.0),
        pos2(10.0, 2.0),
        pos2(20.0, 2.0),
    ];
    let path = EdgePath::try_polyline(points)
        .unwrap()
        .with_corner_radius(CornerRadius::try_from(100.0).unwrap())
        .unwrap();
    assert!(path.points().windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn rounding_is_symmetric_under_path_reversal_at_different_scales_and_angles() {
    for scale in [1e-15, 1.0, 1e15] {
        for degrees in [0.001_f32, 15.0, 30.0, 45.0, 90.0, 135.0, 179.999] {
            let (sin, cos) = degrees.to_radians().sin_cos();
            let points = [
                pos2(-100.0 * scale, 0.0),
                Pos2::ZERO,
                pos2(100.0 * cos * scale, 100.0 * sin * scale),
            ];
            let reversed = [points[2], points[1], points[0]];
            let radius = CornerRadius::try_from(20.0 * scale).unwrap();
            let forward = EdgePath::try_polyline(points)
                .unwrap()
                .with_corner_radius(radius)
                .unwrap();
            let backward = EdgePath::try_polyline(reversed)
                .unwrap()
                .with_corner_radius(radius)
                .unwrap();
            assert_eq!(forward.points().len(), backward.points().len());
            for (a, b) in forward.points().iter().zip(backward.points().iter().rev()) {
                // Относительная погрешность координат, не квадратов расстояний, сохраняет масштаб проверки.
                assert!(
                    (a.x - b.x).abs().max((a.y - b.y).abs()) <= 0.000_1 * scale,
                    "scale={scale}, degrees={degrees}: {a:?} vs {b:?}"
                );
            }
        }
    }
}

#[test]
fn nearly_straight_corner_does_not_lose_small_offsets_with_large_radius() {
    let points = [pos2(-100.0, 0.0), Pos2::ZERO, pos2(100.0, 1e-8)];
    let path = EdgePath::try_polyline(points)
        .unwrap()
        .with_corner_radius(CornerRadius::try_from(f32::MAX).unwrap())
        .unwrap();
    assert_eq!(path.points().first(), points.first());
    assert_eq!(path.points().last(), points.last());
    assert!(
        path.points().iter().all(|point| point.x >= -100.0
            && point.x <= 100.0
            && point.y >= 0.0
            && point.y <= 1e-8)
    );
    assert!(path.points().len() > points.len());
}

#[test]
fn smooth_vertex_metadata_preserves_real_reversals_and_excludes_endpoints() {
    let sharp = EdgePath::try_polyline([
        pos2(-100.0, 0.0),
        Pos2::ZERO,
        pos2(0.0, 100.0),
        pos2(0.0, 50.0),
    ])
    .unwrap();
    assert!((0..sharp.points().len()).all(|index| !sharp.is_smooth_vertex(index)));
    let rounded = sharp
        .with_corner_radius(CornerRadius::try_from(20.0).unwrap())
        .unwrap();
    assert!(!rounded.is_smooth_vertex(0));
    assert!(!rounded.is_smooth_vertex(rounded.points().len() - 1));
    assert!(!rounded.is_smooth_vertex(usize::MAX));
    for (index, &point) in rounded
        .points()
        .iter()
        .enumerate()
        .skip(1)
        .take(rounded.points().len() - 2)
    {
        assert_eq!(rounded.is_smooth_vertex(index), point != pos2(0.0, 100.0));
    }
}

#[test]
fn unrounded_curve_borrows_original_points_without_changing_the_rounded_path() {
    let points: Box<[_]> = [
        pos2(0.0, 0.0),
        pos2(0.0, 0.0),
        pos2(100.0, 0.0),
        pos2(100.0, 100.0),
        pos2(50.0, 100.0),
    ]
    .into();
    let original = points.as_ptr();
    let expected = points.clone();
    let rounded = EdgePath::try_polyline(points)
        .unwrap()
        .with_corner_radius(CornerRadius::try_from(20.0).unwrap())
        .unwrap();
    let Curve::Polyline(sharp) = rounded.unrounded_curve() else {
        panic!("Expected a polyline")
    };
    assert_eq!(sharp, &*expected);
    assert_eq!(sharp.as_ptr(), original);
    assert!(rounded.points().len() > sharp.len());
    assert!(!rounded.points().contains(&pos2(100.0, 0.0)));
    let rounded_again = rounded
        .with_corner_radius(CornerRadius::try_from(5.0).unwrap())
        .unwrap();
    let Curve::Polyline(sharp) = rounded_again.unrounded_curve() else {
        panic!("Expected a polyline")
    };
    assert_eq!(sharp, &*expected);
    assert_eq!(sharp.as_ptr(), original);
}

#[test]
fn unrounded_curve_preserves_lines_polylines_and_beziers() {
    let points = [pos2(0.0, 0.0), pos2(50.0, 80.0), pos2(100.0, 0.0)];
    for path in [
        EdgePath::try_line(points[0], points[2]).unwrap(),
        EdgePath::try_polyline(points).unwrap(),
        EdgePath::try_quadratic(points).unwrap(),
        EdgePath::try_cubic([points[0], points[1], points[1], points[2]]).unwrap(),
    ] {
        match (path.curve(), path.unrounded_curve()) {
            (Curve::Polyline(a), Curve::Polyline(b)) => assert!(std::ptr::eq(a, b)),
            (Curve::Quadratic(a), Curve::Quadratic(b)) => assert_eq!(a, b),
            (Curve::Cubic(a), Curve::Cubic(b)) => assert_eq!(a, b),
            _ => panic!("Unrounded geometry must preserve the original curve kind"),
        }
    }
}
