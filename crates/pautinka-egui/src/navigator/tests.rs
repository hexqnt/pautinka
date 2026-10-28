use egui::pos2;

use super::*;

#[test]
fn corners_and_small_viewports_keep_the_panel_inside_the_canvas() {
    for size in [Vec2::new(800.0, 600.0), Vec2::splat(80.0)] {
        let viewport = Rect::from_min_size(pos2(40.0, 60.0), size);
        for corner in [
            NavigatorCorner::TopLeft,
            NavigatorCorner::TopRight,
            NavigatorCorner::BottomLeft,
            NavigatorCorner::BottomRight,
        ] {
            let nav = Navigator::new(viewport, Rect::ZERO, corner).unwrap();
            assert!(viewport.contains_rect(nav.panel));
            assert_eq!(nav.target(nav.thumbnail.center()), Pos2::ZERO);
            assert_eq!(nav.target(viewport.max), Pos2::ZERO);
        }
    }
    assert!(Navigator::new(Rect::ZERO, Rect::ZERO, NavigatorCorner::TopRight).is_none());
}

#[test]
fn targets_roundtrip_and_clamp_to_graph_bounds() {
    let bounds = Rect::from_min_max(pos2(-400.0, -200.0), pos2(600.0, 300.0));
    let nav = Navigator::new(
        Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
        bounds,
        NavigatorCorner::TopRight,
    )
    .unwrap();
    for point in [bounds.min, bounds.center(), bounds.max] {
        assert!(
            nav.target(nav.camera.to_screen(point, nav.thumbnail))
                .distance(point)
                < 0.001
        );
    }
    assert_eq!(nav.target(nav.panel.min - Vec2::splat(100.0)), bounds.min);
    assert_eq!(nav.target(nav.panel.max + Vec2::splat(100.0)), bounds.max);
}

#[test]
fn marker_clips_and_preserves_minimum_size() {
    let thumbnail = Rect::from_min_size(pos2(10.0, 20.0), Vec2::splat(100.0));
    let camera = Camera {
        center: Pos2::ZERO,
        zoom: 1.0,
        fit: false,
    };
    assert_eq!(
        viewport_marker(
            camera,
            thumbnail,
            Rect::from_center_size(Pos2::ZERO, Vec2::splat(200.0))
        ),
        Some(thumbnail)
    );
    let small = viewport_marker(
        camera,
        thumbnail,
        Rect::from_min_size(pos2(49.0, 49.0), Vec2::splat(1.0)),
    )
    .unwrap();
    assert_eq!(small.size(), Vec2::splat(4.0));
    assert!(thumbnail.contains_rect(small));
    assert!(
        viewport_marker(
            camera,
            thumbnail,
            Rect::from_min_size(pos2(200.0, 200.0), Vec2::splat(10.0))
        )
        .is_none()
    );
}

#[test]
fn edge_geometry_preserves_curves_and_polyline_corners() {
    let camera = Camera::default();
    let thumbnail = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
    let points = [pos2(-10.0, 0.0), pos2(0.0, 20.0), pos2(10.0, 0.0)];
    let shapes: Vec<_> =
        edge_shapes(Curve::Quadratic(points), Color32::RED, camera, thumbnail).collect();
    let [Shape::QuadraticBezier(curve)] = shapes.as_slice() else {
        panic!("Expected a quadratic curve")
    };
    assert_eq!(curve.points, points.map(|p| camera.to_screen(p, thumbnail)));
    let cubic = [points[0], points[1], pos2(5.0, -10.0), points[2]];
    let shapes: Vec<_> =
        edge_shapes(Curve::Cubic(cubic), Color32::RED, camera, thumbnail).collect();
    let [Shape::CubicBezier(curve)] = shapes.as_slice() else {
        panic!("Expected a cubic curve")
    };
    assert_eq!(curve.points, cubic.map(|p| camera.to_screen(p, thumbnail)));
    let shapes: Vec<_> =
        edge_shapes(Curve::Polyline(&points), Color32::RED, camera, thumbnail).collect();
    assert_eq!(shapes.len(), 2);
    for (shape, pair) in shapes.iter().zip(points.windows(2)) {
        let Shape::LineSegment { points, .. } = shape else {
            panic!("Expected a segment")
        };
        assert_eq!(
            *points,
            [
                camera.to_screen(pair[0], thumbnail),
                camera.to_screen(pair[1], thumbnail)
            ]
        );
    }
}

#[test]
fn batches_preserve_order_opacity_and_allow_context_access_in_the_source() {
    let context = egui::Context::default();
    let mut output = context.run_ui(egui::RawInput::default(), |ui| {
        let mut painter = ui.painter().clone();
        painter.set_opacity(0.5);
        paint_shapes(
            &painter,
            (0..130).map(|index| {
                // Обращение к Context внутри ленивого источника не должно оказаться под блокировкой Painter.
                let radius = ui.ctx().pixels_per_point();
                Shape::circle_filled(pos2(index as f32, 10.0), radius, Color32::RED)
            }),
        );
    });
    output.textures_delta.clear();
    let circles: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            Shape::Circle(circle) => Some(circle),
            _ => None,
        })
        .collect();
    assert_eq!(circles.len(), 130);
    for (index, circle) in circles.into_iter().enumerate() {
        assert_eq!(circle.center, pos2(index as f32, 10.0));
        assert_eq!(circle.fill, Color32::RED.gamma_multiply(0.5));
    }
}
