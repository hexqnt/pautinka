use super::*;

#[test]
fn standalone_canvas_selects_nodes_and_focuses_hidden_loops() {
    let mut harness = canvas_harness(State::default());
    harness.get_by_label("Graph canvas");
    click_label(&mut harness, "First");
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));

    click_label(&mut harness, "Loops");
    assert!(!harness.state().canvas.show_loops());
    click_label(&mut harness, "Focus loop");
    assert!(harness.state().canvas.show_loops());
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(1)));
    let viewport = harness.get_by_label("Graph canvas").rect();
    let state = harness.state();
    let center = state
        .canvas
        .camera
        .to_screen(state.graph.edges[1].bounds().center(), viewport);
    assert!(center.distance(viewport.center()) < 1.0);

    click_label(&mut harness, "Loops");
    assert_eq!(harness.state().canvas.selected(), None);
}

#[test]
fn selection_survives_reordering_and_is_cleared_when_the_identity_is_removed() {
    let mut harness = canvas_harness(State::default());
    click_label(&mut harness, "First");
    let selected = harness.state().canvas.selected_id();
    let graph = &mut harness.state_mut().graph;
    graph.nodes.swap(0, 1);
    graph.node_ids.swap(0, 1);
    graph.revision = graph.revision.next_topology();
    harness.run();
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(1)));
    assert_eq!(harness.state().canvas.selected_id(), selected);
    let graph = &mut harness.state_mut().graph;
    graph.nodes.pop();
    graph.node_ids.pop();
    graph.revision = graph.revision.next_topology();
    harness.run();
    assert_eq!(harness.state().canvas.selected(), None);
    assert_eq!(harness.state().canvas.selected_id(), None);
    assert!(
        harness
            .state()
            .events_seen
            .contains(&pautinka_egui::CanvasEvent::SelectionChanged(None))
    );
}

#[test]
fn polyline_selection_uses_the_drawn_segments_and_resize_preserves_the_camera() {
    let mut state = State::default();
    state.graph.edges[0] = EdgePath::try_polyline([
        state.graph.nodes[0],
        pos2(-100.0, 100.0),
        pos2(100.0, 100.0),
        state.graph.nodes[1],
    ])
    .unwrap();
    let mut harness = canvas_harness(state);
    let viewport = harness.get_by_label("Graph canvas").rect();
    let point = harness
        .state()
        .canvas
        .camera
        .to_screen(pos2(0.0, 100.0), viewport);
    pointer_gesture(
        &mut harness,
        point,
        point,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(0)));
    harness.state_mut().canvas.camera.zoom *= 2.0;
    let camera = harness.state().canvas.camera;
    harness.set_size(Vec2::new(1200.0, 800.0));
    harness.run();
    assert_eq!(harness.state().canvas.camera.center, camera.center);
    assert_eq!(harness.state().canvas.camera.zoom, camera.zoom);
}

#[test]
fn hover_and_context_target_follow_the_same_edge_geometry_as_selection() {
    let mut harness = canvas_harness(State::default());
    let viewport = harness.get_by_label("Graph canvas").rect();
    let point = harness
        .state()
        .canvas
        .camera
        .to_screen(pos2(0.0, 0.0), viewport);
    harness.hover_at(point);
    harness.run();
    let expected = Some(pautinka_egui::ElementId::Edge(pautinka_egui::EdgeId::new(
        0,
    )));
    assert_eq!(harness.state().canvas.hovered(), expected);
    pointer_gesture(
        &mut harness,
        point,
        point,
        egui::PointerButton::Secondary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.context_target(), expected);
    assert_eq!(harness.state().canvas.selected(), None);
}

#[test]
fn activation_and_context_events_include_world_coordinates_and_canvas_response_transform() {
    #[derive(Default)]
    struct ResponseState {
        state: State,
        response: Option<pautinka_egui::CanvasResponse>,
    }
    let mut harness = Harness::builder()
        .with_step_dt(1.0 / 60.0)
        .with_max_steps(120)
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, s: &mut ResponseState| {
                s.response = Some(s.state.canvas.show_with(
                    ui,
                    &s.state.graph,
                    &pautinka_egui::DefaultRenderer,
                ));
                s.state
                    .events_seen
                    .extend_from_slice(s.state.canvas.events());
            },
            ResponseState::default(),
        );
    harness.get_by_label("First").click();
    harness.step();
    harness.get_by_label("First").click();
    harness.step();
    assert!(
        harness
            .state()
            .state
            .events_seen
            .iter()
            .any(|event| matches!(
                event,
                pautinka_egui::CanvasEvent::Activated {
                    element: pautinka_egui::ElementId::Node(_),
                    ..
                }
            )),
        "{:?}",
        harness.state().state.events_seen
    );
    harness.get_by_label("Second").click_secondary();
    harness.run();
    let state = harness.state();
    let response = state.response.as_ref().unwrap();
    let world = state.state.graph.nodes[1];
    assert!(response.to_world(response.to_screen(world)).distance(world) < 0.001);
    assert!(response.context_position.unwrap().distance(world) < 0.001);
    assert!(state.state.events_seen.iter().any(|event| matches!(
        event,
        pautinka_egui::CanvasEvent::ContextPositionRequested {
            target: Some(pautinka_egui::ElementId::Node(_)),
            ..
        }
    )));
}

#[test]
fn accessibility_activation_works_with_the_pointer_over_a_different_object() {
    let mut harness = canvas_harness(State::default());
    harness.get_by_label("Second").hover();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "First")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));
    assert!(harness.state().events_seen.iter().any(|event| matches!(event, pautinka_egui::CanvasEvent::Activated { element: pautinka_egui::ElementId::Node(id), .. } if *id == pautinka_egui::NodeId::new(0))));
}
