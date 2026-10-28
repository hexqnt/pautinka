use super::*;
use pautinka_egui::{NodeShape, ShapeKind, geometry::Camera};

fn shapes() -> Vec<NodeShape> {
    let size = Vec2::new(100.0, 80.0);
    let mut result = vec![
        NodeShape::circle(40.0).unwrap(),
        NodeShape::ellipse(size).unwrap(),
    ];
    for shape in [
        NodeShape::rectangle(size).unwrap(),
        NodeShape::triangle(size).unwrap(),
        NodeShape::diamond(size).unwrap(),
    ]
    .into_iter()
    .chain((3..=8).map(|sides| NodeShape::regular_polygon(50.0, sides).unwrap()))
    {
        result.push(shape);
        result.push(shape.with_corner_radius(20.0.try_into().unwrap()));
    }
    result
}

#[test]
fn built_in_shapes_paint_and_select_by_contour_with_the_default_renderer() {
    for shape in shapes() {
        let mut state = State::default();
        state.graph.node_shape = Some(shape);
        state.canvas.set_labels(false);
        state.canvas.set_camera(Camera {
            center: Pos2::ZERO,
            zoom: 2.0,
            fit: false,
        });
        let mut harness = canvas_harness(state);
        harness.run();
        let center = harness.get_by_label("First").rect().center();
        let painted = harness
            .output()
            .shapes
            .iter()
            .any(|clipped| match &clipped.shape {
                egui::Shape::Circle(circle) => {
                    shape.kind() == ShapeKind::Circle && circle.center == center
                }
                egui::Shape::Ellipse(ellipse) => {
                    shape.kind() == ShapeKind::Ellipse && ellipse.center == center
                }
                egui::Shape::Rect(rect) => {
                    shape.kind() == ShapeKind::Rectangle
                        && shape.corner_radius().get() == 0.0
                        && rect.rect.center() == center
                }
                egui::Shape::Path(path) => {
                    path.closed
                        && path.fill == harness.state().graph.node_color
                        && path.points.len() >= 3
                }
                _ => false,
            });
        assert!(painted, "{shape:?}");
        let viewport = harness.get_by_label("Graph canvas").rect();
        let position = harness.state().graph.nodes[0];
        let inside = position + Vec2::new(0.0, -5.0);
        let outside_fraction =
            if shape.kind() == ShapeKind::Rectangle && shape.corner_radius().get() == 0.0 {
                0.6
            } else {
                0.49
            };
        let outside = position + shape.size() * Vec2::new(outside_fraction, -outside_fraction);
        for (world, expected) in [(inside, Some(Selection::Node(0))), (outside, None)] {
            let screen = harness.state().canvas.camera.to_screen(world, viewport);
            harness.hover_at(screen);
            harness.run();
            assert_eq!(
                harness.state().canvas.pick(world, &harness.state().graph),
                expected,
                "{shape:?}"
            );
            assert_eq!(
                harness.state().canvas.hovered(),
                expected.map(|_| pautinka_egui::ElementId::Node(pautinka_egui::NodeId::new(0))),
                "{shape:?}"
            );
            pointer_gesture(
                &mut harness,
                screen,
                screen,
                egui::PointerButton::Primary,
                egui::Modifiers::NONE,
            );
            assert_eq!(harness.state().canvas.selected(), expected, "{shape:?}");
        }
    }
}

#[test]
fn small_shapes_keep_a_minimum_target_and_precise_contours_at_large_positions() {
    let origin = pos2(1e9, -1e9);
    for zoom in [0.5, 1.0, 4.0] {
        let mut state = State::default();
        state.graph.nodes[0] = origin;
        state.graph.node_shape = Some(NodeShape::diamond(Vec2::splat(2.0)).unwrap());
        state.canvas.set_camera(Camera {
            center: origin,
            zoom,
            fit: false,
        });
        let mut harness = canvas_harness(state);
        harness.run();
        let center = harness.get_by_label("First").rect().center();
        for (screen, expected) in [
            (center + Vec2::new(5.0, 0.0), Some(Selection::Node(0))),
            (center + Vec2::new(8.0, -8.0), None),
        ] {
            pointer_gesture(
                &mut harness,
                screen,
                screen,
                egui::PointerButton::Primary,
                egui::Modifiers::NONE,
            );
            assert_eq!(harness.state().canvas.selected(), expected, "zoom = {zoom}");
        }
    }
}
