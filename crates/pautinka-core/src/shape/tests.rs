use super::*;
use crate::pos2;

fn shapes(scale: f32) -> Vec<NodeShape> {
    let size = Vec2::new(80.0, 60.0) * scale;
    let mut shapes = vec![
        NodeShape::circle(30.0 * scale).unwrap(),
        NodeShape::ellipse(size).unwrap(),
    ];
    for shape in [
        NodeShape::rectangle(size).unwrap(),
        NodeShape::triangle(size).unwrap(),
        NodeShape::diamond(size).unwrap(),
    ]
    .into_iter()
    .chain((3..=12).map(|sides| NodeShape::regular_polygon(40.0 * scale, sides).unwrap()))
    {
        shapes.push(shape);
        shapes.push(shape.with_corner_radius((8.0 * scale).try_into().unwrap()));
        shapes.push(shape.with_corner_radius((1000.0 * scale).try_into().unwrap()));
    }
    shapes
}

#[test]
fn invalid_dimensions_and_side_counts_are_rejected() {
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MAX] {
        assert!(NodeShape::circle(radius).is_err());
        assert!(NodeShape::regular_polygon(radius, 6).is_err());
    }
    for size in [
        Vec2::ZERO,
        Vec2::new(-1.0, 2.0),
        Vec2::new(2.0, f32::NAN),
        Vec2::new(f32::INFINITY, 1.0),
    ] {
        for constructor in [
            NodeShape::ellipse,
            NodeShape::rectangle,
            NodeShape::triangle,
            NodeShape::diamond,
        ] {
            assert!(constructor(size).is_err());
        }
    }
    for sides in 0..3 {
        assert!(NodeShape::regular_polygon(10.0, sides).is_err());
    }
    assert!(NodeShape::regular_polygon(10.0, u16::MAX).is_ok());
    for radius in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(CornerRadius::try_from(radius).is_err());
    }
}

#[test]
fn contours_bounds_and_anchors_agree_across_scales_and_rounding() {
    for scale in [1e-25, 0.0001, 1.0, 10000.0, 1e25] {
        for shape in shapes(scale) {
            let mut contour = Vec::new();
            shape.outline(|point| contour.push(point));
            assert!(contour.len() >= 3);
            assert!(contour.len() <= shape.outline_capacity());
            assert_ne!(contour.first(), contour.last());
            assert!(contour.windows(2).all(|pair| pair[0] != pair[1]));
            let bounds = shape
                .bounds(Pos2::ZERO)
                .expand(shape.size().max_elem() * 1e-6);
            for &point in &contour {
                assert!(point.is_finite(), "{shape:?}");
                assert!(bounds.contains(point), "{shape:?}: {point:?}");
                assert!(shape.contains(Pos2::ZERO, point), "{shape:?}: {point:?}");
            }
            assert!(shape.contains(Pos2::ZERO, Pos2::ZERO), "{shape:?}");
            for step in 0..144 {
                let angle = std::f32::consts::TAU * step as f32 / 144.0;
                let direction = Vec2::angled(angle);
                let anchor = shape.anchor(Pos2::ZERO, direction).unwrap();
                let point = anchor.position();
                assert!(
                    point.is_finite() && point != Pos2::ZERO,
                    "{shape:?}: {point:?}"
                );
                assert!(shape.contains(Pos2::ZERO, point), "{shape:?}: {point:?}");
                assert!(
                    shape.contains(Pos2::ZERO, point * 0.999),
                    "{shape:?}: {point:?}"
                );
                assert!(
                    !shape.contains(Pos2::ZERO, point * 1.001),
                    "{shape:?}: {point:?}"
                );
                assert!((anchor.direction() - direction).length() < 1e-6);
            }
        }
    }
}

#[test]
fn rounding_removes_corners_and_anchors_follow_circular_arcs() {
    let square = NodeShape::rectangle(Vec2::splat(100.0)).unwrap();
    let rounded = square.with_corner_radius(20.0.try_into().unwrap());
    assert!(square.contains(Pos2::ZERO, pos2(49.0, -49.0)));
    assert!(!rounded.contains(Pos2::ZERO, pos2(49.0, -49.0)));
    assert!(rounded.contains(Pos2::ZERO, pos2(30.0, -50.0)));
    let anchor = rounded.anchor(Pos2::ZERO, Vec2::new(1.0, -1.0)).unwrap();
    let expected = 30.0 + 20.0 / 2.0_f32.sqrt();
    assert!((anchor.position() - pos2(expected, -expected)).length() < 1e-5);
    let fully_rounded = square.with_corner_radius(1000.0.try_into().unwrap());
    let circle = NodeShape::circle(50.0).unwrap();
    for direction in [Vec2::X, Vec2::Y, Vec2::new(1.0, 1.0)] {
        assert!(
            (fully_rounded
                .anchor(Pos2::ZERO, direction)
                .unwrap()
                .position()
                - circle.anchor(Pos2::ZERO, direction).unwrap().position())
            .length()
                < 1e-5
        );
    }
}

#[test]
fn ellipses_polygons_and_triangles_have_the_expected_geometry() {
    let ellipse = NodeShape::ellipse(Vec2::new(80.0, 40.0)).unwrap();
    assert!(ellipse.contains(Pos2::ZERO, pos2(40.0, 0.0)));
    assert!(ellipse.contains(Pos2::ZERO, pos2(0.0, 20.0)));
    assert!(!ellipse.contains(Pos2::ZERO, pos2(35.0, 15.0)));
    let triangle = NodeShape::triangle(Vec2::new(80.0, 60.0)).unwrap();
    assert!(triangle.contains(Pos2::ZERO, pos2(0.0, -30.0)));
    assert!(triangle.contains(Pos2::ZERO, pos2(40.0, 30.0)));
    assert!(!triangle.contains(Pos2::ZERO, pos2(30.0, -20.0)));
    let diamond = NodeShape::diamond(Vec2::new(80.0, 40.0)).unwrap();
    assert!(diamond.contains(Pos2::ZERO, pos2(20.0, 10.0)));
    assert!(!diamond.contains(Pos2::ZERO, pos2(30.0, 15.0)));
    for sides in 3..=12 {
        let polygon = NodeShape::regular_polygon(30.0, sides).unwrap();
        let mut vertices = Vec::new();
        polygon.outline(|point| vertices.push(point));
        assert_eq!(vertices.len(), usize::from(sides));
        let length = vertices[0].distance(vertices[1]);
        for (&a, &b) in vertices.iter().zip(vertices.iter().cycle().skip(1)) {
            assert!((a.to_vec2().length() - 30.0).abs() < 1e-5);
            assert!((a.distance(b) - length).abs() < 1e-5);
        }
    }
}

#[test]
fn local_precision_and_extreme_aspect_ratios_are_preserved() {
    let origin = pos2(1e9, -1e9);
    let shape = NodeShape::diamond(Vec2::new(2.0, 1.0)).unwrap();
    assert!(shape.contains_precise(origin, HitPoint::new(1e9 + 0.25, -1e9 + 0.25).unwrap()));
    assert!(!shape.contains_precise(origin, HitPoint::new(1e9 + 0.75, -1e9 + 0.25).unwrap()));
    for size in [Vec2::new(1e-25, 1e25), Vec2::new(1e25, 1e-25)] {
        for shape in [
            NodeShape::ellipse(size).unwrap(),
            NodeShape::rectangle(size).unwrap(),
            NodeShape::triangle(size).unwrap(),
            NodeShape::diamond(size).unwrap(),
        ] {
            for shape in [shape, shape.with_corner_radius(1e25.try_into().unwrap())] {
                let bounds = shape.bounds(Pos2::ZERO);
                shape.outline(|point| {
                    assert!(bounds.contains(point), "{shape:?}: {point:?}");
                    assert!(shape.contains(Pos2::ZERO, point), "{shape:?}: {point:?}");
                });
                for direction in [Vec2::X, Vec2::Y, -Vec2::Y, Vec2::new(1.0, 1.0)] {
                    let point = shape.anchor(Pos2::ZERO, direction).unwrap().position();
                    assert!(point.is_finite());
                    assert_ne!(point, Pos2::ZERO, "{shape:?}: {direction:?}");
                    assert!(shape.contains(Pos2::ZERO, point), "{shape:?}: {point:?}");
                    assert!(
                        !shape.contains(Pos2::ZERO, point * 1.001),
                        "{shape:?}: ({:e}, {:e}), direction={direction:?}",
                        point.x,
                        point.y
                    );
                }
            }
        }
    }
    assert!(!shape.contains(Pos2::ZERO, pos2(f32::NAN, 0.0)));
    assert!(!shape.contains(Pos2::ZERO, pos2(f32::INFINITY, 0.0)));
    assert!(shape.anchor(pos2(f32::MAX, 0.0), Vec2::X).is_ok());
    assert!(shape.anchor(Pos2::ZERO, Vec2::ZERO).is_err());
    assert!(shape.anchor(Pos2::ZERO, Vec2::new(f32::NAN, 0.0)).is_err());
}
