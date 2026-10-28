use super::*;

#[derive(Default)]
struct DiamondRenderer {
    revision: u64,
}
impl pautinka_egui::Renderer for DiamondRenderer {
    fn revision(&self) -> u64 {
        self.revision
    }
    fn node_geometry(
        &self,
        _: usize,
        _: Node<'_>,
        _: pautinka_egui::NodeGeometry,
    ) -> pautinka_egui::NodeGeometry {
        pautinka_egui::NodeGeometry::world(Rect::from_center_size(
            pos2(300.0, 0.0),
            Vec2::splat(100.0 * (self.revision + 1) as f32),
        ))
        .unwrap()
    }
    fn node_hit_test(
        &self,
        _: usize,
        node: Node<'_>,
        _: pautinka_egui::NodeGeometry,
        point: pautinka_egui::HitPoint,
        _: f32,
    ) -> Option<bool> {
        let delta = point.relative_to(node.position) - Vec2::new(300.0, 0.0);
        Some(delta.x.abs() + delta.y.abs() <= 50.0 * (self.revision + 1) as f32)
    }
    fn paint_node(
        &self,
        context: &pautinka_egui::RenderContext<'_>,
        _: Node<'_>,
        visual: &pautinka_egui::NodeVisual,
    ) {
        let r = visual.rect;
        context.painter.add(egui::Shape::convex_polygon(
            vec![
                r.center_top(),
                r.right_center(),
                r.center_bottom(),
                r.left_center(),
            ],
            visual.style.fill,
            visual.style.stroke,
        ));
    }
}

#[test]
fn translucent_node_fill_and_border_keep_their_layering_and_click_selection() {
    use egui::{Shape, Stroke};
    use pautinka_egui::NodeShape;

    for node_shape in [
        None,
        Some(NodeShape::rectangle(Vec2::new(40.0, 20.0)).unwrap()),
        Some(NodeShape::circle(20.0).unwrap()),
        Some(NodeShape::ellipse(Vec2::new(40.0, 20.0)).unwrap()),
        Some(NodeShape::triangle(Vec2::new(40.0, 30.0)).unwrap()),
        Some(NodeShape::diamond(Vec2::new(40.0, 30.0)).unwrap()),
        Some(
            NodeShape::regular_polygon(20.0, 6)
                .unwrap()
                .with_corner_radius(5.0.try_into().unwrap()),
        ),
        Some(
            NodeShape::rectangle(Vec2::new(40.0, 30.0))
                .unwrap()
                .with_corner_radius(5.0.try_into().unwrap()),
        ),
    ] {
        for (fill_alpha, border_alpha, opacity) in [
            (255, 255, 1.0),
            (128, 255, 1.0),
            (255, 128, 1.0),
            (128, 128, 1.0),
            (255, 255, 0.5),
        ] {
            let mut state = State::default();
            state.graph.node_shape = node_shape;
            state.graph.node_color = Color32::from_rgba_unmultiplied(143, 47, 197, fill_alpha);
            state.canvas.style.background =
                Color32::from_rgba_unmultiplied(19, 31, 53, border_alpha);
            state.canvas.set_labels(false);
            let fill = state.graph.node_color.gamma_multiply(opacity);
            let stroke = Stroke::new(1.0, state.canvas.style.background.gamma_multiply(opacity));
            let mut harness = Harness::builder()
                .with_size(Vec2::new(800.0, 600.0))
                .build_ui_state(
                    move |ui, state| {
                        ui.set_opacity(opacity);
                        State::show(state, ui);
                    },
                    state,
                );
            harness.run();
            let center = harness.get_by_label("First").rect().center();
            let node_shapes = harness
                .output()
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    Shape::Circle(shape) if shape.center.distance_sq(center) < 0.000_001 => {
                        Some((shape.fill, shape.stroke))
                    }
                    Shape::Rect(shape)
                        if node_shape.is_some()
                            && shape.rect.center().distance_sq(center) < 0.000_001 =>
                    {
                        Some((shape.fill, shape.stroke))
                    }
                    Shape::Ellipse(shape) if shape.center.distance_sq(center) < 0.000_001 => {
                        Some((shape.fill, shape.stroke))
                    }
                    Shape::Path(shape) if Rect::from_points(&shape.points).contains(center) => {
                        let egui::epaint::ColorMode::Solid(color) = shape.stroke.color else {
                            return None;
                        };
                        Some((shape.fill, Stroke::new(shape.stroke.width, color)))
                    }
                    _ => None,
                })
                .filter(|(shape_fill, shape_stroke)| *shape_fill == fill || *shape_stroke == stroke)
                .collect::<Vec<_>>();
            if fill_alpha == 255 && border_alpha == 255 && opacity == 1.0 {
                assert!(node_shapes.contains(&(fill, stroke)));
            } else {
                let fill_position = node_shapes
                    .iter()
                    .position(|(shape_fill, shape_stroke)| {
                        *shape_fill == fill && shape_stroke.is_empty()
                    })
                    .expect("Translucent nodes retain a separate fill");
                let border_position = node_shapes
                    .iter()
                    .position(|(shape_fill, shape_stroke)| {
                        *shape_fill == Color32::TRANSPARENT && *shape_stroke == stroke
                    })
                    .expect("Translucent nodes retain a separate border");
                assert!(fill_position < border_position);
                assert!(!node_shapes.contains(&(fill, stroke)));
            }
            click_label(&mut harness, "First");
            assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));
        }
    }
}

#[test]
fn fitted_large_graph_keeps_vector_edges_and_accessible_nodes() {
    use pautinka_test_support::canvas::{Scene, SceneKind};
    use std::num::NonZeroUsize;

    let scene = Scene::new(SceneKind::Network, NonZeroUsize::new(1000).unwrap());
    let prepared = pautinka_egui::PreparedGraph::new(&scene).unwrap();
    let mut canvas = Canvas::default();
    assert!(canvas.install_prepared(prepared, &scene));
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, (scene, canvas): &mut (Scene, Canvas)| {
                canvas.show(ui, scene);
            },
            (scene, canvas),
        );
    harness.run();
    assert!(harness.state().1.camera.zoom < 0.5);
    assert!(harness.output().shapes.iter().any(|shape| matches!(
        shape.shape,
        egui::Shape::QuadraticBezier(_) | egui::Shape::CubicBezier(_)
    )));
    harness.get_by_label("Node 0");
    harness.get_by_label("Node 999");
    let point = harness.get_by_label("Node 500").rect().center();
    let viewport = harness.get_by_label("Graph canvas").rect();
    let before = harness.state().1.camera;
    let expected = harness
        .state()
        .1
        .pick(before.to_world(point, viewport), &harness.state().0);
    assert!(expected.is_some());
    pointer_gesture(
        &mut harness,
        point,
        point,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().1.selected(), expected);
    assert_eq!(harness.state().1.camera.center, before.center);
    assert_eq!(harness.state().1.camera.zoom, before.zoom);
}

#[test]
fn curved_edge_is_selectable_on_the_true_curve_at_high_zoom_without_repeated_resolve() {
    use pautinka_egui::geometry::Camera;
    let [a, b, c] = [pos2(0.0, 0.0), pos2(500.0, 1000.0), pos2(1000.0, 0.0)];
    let t = 12.5 / 24.0;
    let point = a.lerp(b, t).lerp(b.lerp(c, t), t);
    let mut state = State::default();
    state.graph.edges[0] = EdgePath::try_quadratic([a, b, c]).unwrap();
    state.canvas.set_camera(Camera {
        center: point,
        zoom: 20.0,
        fit: false,
    });
    let mut harness = canvas_harness(state);
    let viewport = harness.get_by_label("Graph canvas").rect();
    let screen = harness.state().canvas.camera.to_screen(point, viewport);
    harness.hover_at(screen);
    for _ in 0..10 {
        harness.step();
    }
    assert_eq!(harness.state().graph.resolve_calls.get(), 0);
    assert_eq!(
        harness.state().canvas.pick(point, &harness.state().graph),
        Some(Selection::Edge(0))
    );
    assert_eq!(
        harness.state().canvas.hovered(),
        Some(pautinka_egui::ElementId::Edge(pautinka_egui::EdgeId::new(
            0
        )))
    );
    pointer_gesture(
        &mut harness,
        screen,
        screen,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(0)));
}

#[test]
fn selected_nodes_keep_domain_fill_color_and_report_accessible_selection() {
    let mut state = State::default();
    state.graph.node_color = Color32::RED;
    let mut harness = canvas_harness(state);
    click_label(&mut harness, "First");
    let node = harness.get_by_label("First");
    let center = node.rect().center();
    assert!(harness.output().shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Circle(c) if c.center == center && c.fill == Color32::RED)));
    assert_eq!(node.accesskit_node().is_selected(), Some(true));
}

#[test]
fn custom_shapes_share_bounds_and_hit_testing_and_renderer_revision_invalidates_cache() {
    #[derive(Default)]
    struct Custom {
        state: State,
        renderer: DiamondRenderer,
    }
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, s: &mut Custom| {
                s.state.canvas.show_with(ui, &s.state.graph, &s.renderer);
            },
            Custom::default(),
        );
    let state = harness.state();
    let inside = state.state.graph.nodes[0] + Vec2::new(260.0, 0.0);
    let outside = state.state.graph.nodes[0] + Vec2::new(260.0, 40.0);
    assert_eq!(
        state
            .state
            .canvas
            .pick_with(inside, &state.state.graph, &state.renderer),
        Some(Selection::Node(0))
    );
    assert_eq!(
        state
            .state
            .canvas
            .pick_with(outside, &state.state.graph, &state.renderer),
        None
    );
    let rebuilds = state.state.canvas.stats().rebuilds;
    let viewport = harness.get_by_label("Graph canvas").rect();
    let inside_screen = state.state.canvas.camera.to_screen(inside, viewport);
    let outside_screen = state.state.canvas.camera.to_screen(outside, viewport);
    pointer_gesture(
        &mut harness,
        inside_screen,
        inside_screen,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(
        harness.state().state.canvas.selected(),
        Some(Selection::Node(0))
    );
    pointer_gesture(
        &mut harness,
        outside_screen,
        outside_screen,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().state.canvas.selected(), None);
    harness.state_mut().renderer.revision += 1;
    harness.run();
    assert_eq!(harness.state().state.canvas.stats().rebuilds, rebuilds + 1);
    let state = harness.state();
    assert_eq!(
        state
            .state
            .canvas
            .pick_with(outside, &state.state.graph, &state.renderer),
        Some(Selection::Node(0))
    );
    let point = harness.get_by_label("Graph canvas").rect().center();
    let response_world = harness
        .state()
        .state
        .canvas
        .camera
        .to_world(point, harness.get_by_label("Graph canvas").rect());
    assert!(response_world.is_finite());
}

#[test]
fn overview_lod_reduces_meshes_without_changing_picking_or_accessible_nodes() {
    let mut state = State::default();
    state.canvas.options.lod.enabled = false;
    let mut harness = canvas_harness(state);
    let full_shapes = harness.output().shapes.len();
    let point = harness.state().graph.edges[0].midpoint();
    let pick = harness.state().canvas.pick(point, &harness.state().graph);
    harness.state_mut().canvas.options.lod.enabled = true;
    harness
        .state_mut()
        .canvas
        .options
        .lod
        .overview_above_elements = 0;
    harness.run();
    assert!(harness.output().shapes.len() < full_shapes);
    assert_eq!(
        harness.state().canvas.pick(point, &harness.state().graph),
        pick
    );
    harness.get_by_role_and_label(egui::accesskit::Role::Button, "First");
    harness.get_by_role_and_label(egui::accesskit::Role::Button, "Second");
}

#[test]
fn arrows_and_hit_tolerances_work_with_extreme_units_and_large_absolute_positions() {
    for (origin, scale) in [
        (Pos2::ZERO, 1e-25),
        (Pos2::ZERO, 1e25),
        (pos2(1e9, 1e9), 1.0),
    ] {
        let mut state = State::default();
        state.graph.nodes = vec![origin, origin + Vec2::new(100.0 * scale, 0.0)];
        state.graph.edges = [
            EdgePath::try_line(state.graph.nodes[0], state.graph.nodes[1]).unwrap(),
            EdgePath::try_line(state.graph.nodes[1], state.graph.nodes[1]).unwrap(),
        ];
        state.graph.bounds_padding = scale;
        state.canvas.options.lod.enabled = false;
        state.canvas.set_labels(false);
        state.canvas.set_show_loops(false, &state.graph);
        let mut harness = canvas_harness(state);
        let viewport = harness.get_by_label("Graph canvas").rect();
        let point = harness
            .state()
            .canvas
            .camera
            .to_screen(harness.state().graph.edges[0].midpoint(), viewport);
        let arrow_segments = harness.output().shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::LineSegment { points, .. } if points[0].distance(points[1]) > 5.0 && points[0].distance(points[1]) < 10.0)).count();
        assert_eq!(arrow_segments, 2);
        pointer_gesture(
            &mut harness,
            point,
            point,
            egui::PointerButton::Primary,
            egui::Modifiers::NONE,
        );
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(0)));
        let outside = point + Vec2::new(0.0, 20.0);
        pointer_gesture(
            &mut harness,
            outside,
            outside,
            egui::PointerButton::Primary,
            egui::Modifiers::NONE,
        );
        assert_eq!(harness.state().canvas.selected(), None);
    }
}

#[test]
fn overview_nodes_without_strokes_use_one_paint_layer() {
    use egui::Shape;
    use pautinka_egui::NodeShape;

    for shape in [None, Some(NodeShape::triangle(Vec2::splat(20.0)).unwrap())] {
        let mut state = State::default();
        state.graph.node_shape = shape;
        state.canvas.options.lod.overview_above_elements = 0;
        state.canvas.set_labels(false);
        let mut harness = canvas_harness(state);
        harness.run();
        let center = harness.get_by_label("First").rect().center();
        let count = harness
            .output()
            .shapes
            .iter()
            .filter(|clipped| match &clipped.shape {
                Shape::Circle(circle) => circle.center.distance_sq(center) < 0.000_001,
                Shape::Path(path) => {
                    Rect::from_points(&path.points).center().distance_sq(center) < 0.000_001
                }
                _ => false,
            })
            .count();
        assert_eq!(count, 1);
    }
}
