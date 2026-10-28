use super::*;

#[test]
fn wheel_zooms_smoothly_over_nodes_and_background_and_keeps_the_pointer_anchor() {
    use egui::{Event, Modifiers, MouseWheelSource, MouseWheelUnit, TouchPhase};

    for over_node in [false, true] {
        for modifiers in [Modifiers::NONE, Modifiers::CTRL] {
            let mut harness = Harness::builder()
                .with_size(Vec2::new(800.0, 600.0))
                .with_max_steps(120)
                .build_ui_state(|ui, state| State::show(state, ui), State::default());
            let viewport = harness.get_by_label("Graph canvas").rect();
            let pointer = if over_node {
                harness.get_by_label("First").rect().center()
            } else {
                viewport.center() + Vec2::new(0.0, 100.0)
            };
            harness.hover_at(pointer);
            harness.run();
            let before = harness.state().canvas.camera;
            let anchor = before.to_world(pointer, viewport);
            harness.event_modifiers(
                Event::MouseWheel {
                    unit: MouseWheelUnit::Point,
                    delta: Vec2::new(0.0, 100.0),
                    phase: TouchPhase::Move,
                    source: MouseWheelSource::Unknown,
                    modifiers,
                },
                modifiers,
            );
            harness.step();
            let after = harness.state().canvas.camera;
            let expected_factor = if modifiers.ctrl { 0.5_f32 } else { 0.3_f32 }.exp();
            assert!(after.zoom > before.zoom);
            assert!(after.zoom / before.zoom < expected_factor);
            assert!(after.to_screen(anchor, viewport).distance(pointer) < 0.001);
            harness.step();
            assert!(harness.state().canvas.camera.zoom > after.zoom);
            harness.run();
            let settled = harness.state().canvas.camera;
            assert!(
                (settled.zoom / before.zoom - expected_factor).abs() < 0.0001,
                "over_node={over_node}, ctrl={}, before={}, settled={}",
                modifiers.ctrl,
                before.zoom,
                settled.zoom
            );
            assert!(settled.to_screen(anchor, viewport).distance(pointer) < 0.001);
            harness.run();
            assert_eq!(harness.state().canvas.camera.zoom, settled.zoom);
            assert_eq!(harness.state().canvas.camera.center, settled.center);
        }
    }
}

#[test]
fn zoom_out_stops_at_graph_fit_and_tracks_viewport_size() {
    use egui::{Event, Modifiers, MouseWheelSource, MouseWheelUnit, TouchPhase};
    use pautinka_egui::geometry::Camera;

    let mut harness = canvas_harness(State::default());
    let viewport = harness.get_by_label("Graph canvas").rect();
    harness.state_mut().canvas.camera.zoom *= 2.0;
    harness.hover_at(viewport.center());
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: Vec2::new(0.0, -1000.0),
        phase: TouchPhase::Move,
        source: MouseWheelSource::Unknown,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    let minimum = Camera::fit_zoom(harness.state().graph.bounds(), viewport.shrink(16.0));
    assert!((harness.state().canvas.camera.zoom - minimum).abs() < 0.0001);
    harness.set_size(Vec2::new(1200.0, 800.0));
    harness.step();
    let viewport = harness.get_by_label("Graph canvas").rect();
    let larger_minimum = Camera::fit_zoom(harness.state().graph.bounds(), viewport.shrink(16.0));
    assert!(larger_minimum > minimum);
    assert_eq!(harness.state().canvas.camera.zoom, larger_minimum);
}

#[test]
fn invalid_external_geometry_and_camera_return_errors_without_panicking() {
    use pautinka_egui::CanvasError;
    let mut state = State::default();
    state.graph.nodes[0].x = f32::NAN;
    let mut harness = canvas_harness(state);
    assert!(matches!(
        harness.state().error,
        Some(CanvasError::Spatial(_))
    ));
    harness.state_mut().graph.nodes[0].x = -100.0;
    harness.state_mut().graph.revision = harness.state().graph.revision.next_geometry();
    harness.run();
    assert_eq!(harness.state().error, None);
    harness.state_mut().canvas.camera.zoom = 0.0;
    harness.run();
    assert_eq!(harness.state().error, Some(CanvasError::InvalidCamera));
}

#[test]
fn fit_and_focus_do_not_depend_on_world_units_and_zoom_limits_are_explicit() {
    use pautinka_egui::ZoomRange;
    for scale in [0.0001, 1.0, 10000.0] {
        let mut state = State::default();
        state.graph.nodes.iter_mut().for_each(|p| *p *= scale);
        state.graph.edges = [
            EdgePath::try_line(state.graph.nodes[0], state.graph.nodes[1]).unwrap(),
            EdgePath::try_line(state.graph.nodes[1], state.graph.nodes[1]).unwrap(),
        ];
        state.graph.bounds_padding = scale;
        let mut harness = canvas_harness(state);
        let fit = harness.state().canvas.camera.zoom;
        let viewport = harness.get_by_label("Graph canvas").rect();
        assert!(
            viewport.contains(
                harness
                    .state()
                    .canvas
                    .camera
                    .to_screen(harness.state().graph.nodes[0], viewport)
            )
        );
        assert!(fit * scale > 3.0);
        let state = harness.state_mut();
        state.canvas.focus(Selection::Node(0), &state.graph);
        harness.run();
        assert!(harness.state().canvas.camera.zoom >= fit * 2.0);
        assert_eq!(
            harness.state().canvas.camera.center,
            harness.state().graph.nodes[0]
        );
        harness.state_mut().canvas.options.camera.zoom =
            ZoomRange::new(fit * 0.5, fit * 1.5).unwrap();
        harness.state_mut().canvas.camera.fit = true;
        harness.run();
        assert!(harness.state().canvas.camera.zoom <= fit * 1.5);
    }
}

#[test]
fn focus_centers_custom_screen_geometry_with_an_offset_from_the_model_position() {
    struct OffsetSymbol;
    impl pautinka_egui::Renderer for OffsetSymbol {
        fn node_geometry(
            &self,
            _: usize,
            _: Node<'_>,
            _: pautinka_egui::NodeGeometry,
        ) -> pautinka_egui::NodeGeometry {
            pautinka_egui::NodeGeometry::screen(Rect::from_center_size(
                pos2(300.0, 50.0),
                Vec2::splat(60.0),
            ))
            .unwrap()
        }
    }
    let mut state = State::default();
    state.canvas.focus(Selection::Node(0), &state.graph);
    let harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, s: &mut State| {
                s.canvas.show_with(ui, &s.graph, &OffsetSymbol);
            },
            state,
        );
    let viewport = harness.get_by_label("Graph canvas").rect();
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "First")
            .rect()
            .center()
            .distance(viewport.center())
            < 0.001
    );
}
