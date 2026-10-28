use super::*;
use pautinka_egui::{Camera, CanvasEvent, GridOptions, GridSnapMode, SnapKinds};

const POINTER_OFFSET: Vec2 = Vec2::new(2.0, 1.0);

fn state(mode: GridSnapMode, zoom: f32) -> State {
    let mut state = State::default();
    let mut grid = GridOptions::new(20.0, 16.0).unwrap();
    grid.visible = false;
    state.canvas.options.grid = Some(grid);
    state.canvas.options.grid_snap = mode;
    state.canvas.options.interaction.snapping = true;
    state.canvas.options.snapping.kinds = SnapKinds::NONE;
    state.canvas.options.camera.limit_zoom_out_to_fit = false;
    state.canvas.set_camera(Camera {
        center: Pos2::ZERO,
        zoom,
        fit: false,
    });
    state
}

fn harness(state: State) -> Harness<'static, State> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .with_max_steps(120)
        .build_ui_state(|ui, state| State::show(state, ui), state)
}

fn begin(harness: &mut Harness<'_, State>) {
    let from = harness.get_by_label("First").rect().center() + POINTER_OFFSET;
    harness.hover_at(from);
    harness.event(pointer_button(
        from,
        egui::PointerButton::Primary,
        true,
        egui::Modifiers::NONE,
    ));
    harness.step();
}

fn move_to(harness: &mut Harness<'_, State>, raw: Pos2) -> Pos2 {
    let viewport = harness.get_by_label("Graph canvas").rect();
    let pointer = harness.state().canvas.camera.to_screen(raw, viewport) + POINTER_OFFSET;
    harness.event(egui::Event::PointerMoved(pointer));
    harness.step();
    pointer
}

fn last_position(harness: &Harness<'_, State>) -> Pos2 {
    harness
        .state()
        .events_seen
        .iter()
        .rev()
        .find_map(|event| match event {
            CanvasEvent::NodeDragged { position, .. }
            | CanvasEvent::NodeDragFinished { position, .. } => Some(*position),
            _ => None,
        })
        .expect("Node drag position")
}

fn has_marker(harness: &Harness<'_, State>) -> bool {
    harness.output().shapes.iter().any(|shape| {
        matches!(&shape.shape,
            egui::Shape::Circle(circle) if circle.radius == 5.0 && circle.stroke.width == 1.5
                && circle.stroke.color == harness.state().canvas.style.highlight
        )
    })
}

#[test]
fn both_grid_modes_snap_at_screen_tolerance_with_hidden_or_visible_grid_and_finish_once() {
    for (mode, zoom, step) in [
        (GridSnapMode::Fixed, 0.5, 20.0),
        (GridSnapMode::Fixed, 2.0, 20.0),
        (GridSnapMode::Adaptive, 0.5, 100.0),
        (GridSnapMode::Adaptive, 2.0, 20.0),
    ] {
        for visible in [false, true] {
            let mut state = state(mode, zoom);
            state.canvas.options.grid.as_mut().unwrap().visible = visible;
            let mut harness = harness(state);
            begin(&mut harness);
            let expected = pos2(step, step);
            let raw = expected + Vec2::new(2.0, -1.0) / zoom;
            let pointer = move_to(&mut harness, raw);
            assert!(last_position(&harness).distance(expected) < 0.001);
            assert!(has_marker(&harness));
            assert_eq!(harness.state().canvas.stats().snap_rebuilds, 0);
            harness.event(pointer_button(
                pointer,
                egui::PointerButton::Primary,
                false,
                egui::Modifiers::NONE,
            ));
            harness.run();
            assert!(!has_marker(&harness));
            assert!(last_position(&harness).distance(expected) < 0.001);
            assert_eq!(
                harness
                    .state()
                    .events_seen
                    .iter()
                    .filter(|event| matches!(event, CanvasEvent::NodeDragFinished { .. }))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn missing_grid_options_use_default_steps_without_drawing_the_grid() {
    for (mode, expected) in [
        (GridSnapMode::Fixed, pos2(10.0, 20.0)),
        (GridSnapMode::Adaptive, pos2(50.0, 50.0)),
    ] {
        let mut state = state(mode, 1.0);
        state.canvas.options.grid = None;
        let mut harness = harness(state);
        begin(&mut harness);
        move_to(&mut harness, expected + Vec2::new(1.0, -1.0));
        assert!(last_position(&harness).distance(expected) < 0.001);
        assert!(has_marker(&harness));
        assert_eq!(harness.state().canvas.stats().snap_rebuilds, 0);
    }
}

#[test]
fn grid_hold_survives_visibility_but_resets_on_mode_step_disable_and_cancel() {
    let mut harness = harness(state(GridSnapMode::Fixed, 1.0));
    begin(&mut harness);
    move_to(&mut harness, pos2(24.0, 20.0));
    move_to(&mut harness, pos2(27.0, 20.0));
    assert!(last_position(&harness).distance(pos2(20.0, 20.0)) < 0.001);
    harness
        .state_mut()
        .canvas
        .options
        .grid
        .as_mut()
        .unwrap()
        .visible = true;
    harness.step();
    assert!(has_marker(&harness));
    assert!(last_position(&harness).distance(pos2(20.0, 20.0)) < 0.001);
    harness.state_mut().canvas.options.grid_snap = GridSnapMode::Adaptive;
    harness.step();
    assert!(last_position(&harness).distance(pos2(27.0, 20.0)) < 0.001);
    assert!(!has_marker(&harness));
    let options = &mut harness.state_mut().canvas.options;
    options.grid_snap = GridSnapMode::Fixed;
    options.grid = options.grid.and_then(|grid| grid.with_spacing(30.0, 16.0));
    move_to(&mut harness, pos2(31.0, 30.0));
    move_to(&mut harness, pos2(37.0, 30.0));
    assert!(last_position(&harness).distance(pos2(30.0, 30.0)) < 0.001);
    let grid = &mut harness.state_mut().canvas.options.grid;
    *grid = grid.and_then(|grid| grid.with_spacing(20.0, 16.0));
    harness.step();
    assert!(last_position(&harness).distance(pos2(37.0, 30.0)) < 0.001);
    assert!(!has_marker(&harness));
    move_to(&mut harness, pos2(21.0, 20.0));
    assert!(has_marker(&harness));
    harness.state_mut().canvas.options.interaction.snapping = false;
    harness.step();
    assert!(last_position(&harness).distance(pos2(21.0, 20.0)) < 0.001);
    assert!(!has_marker(&harness));
    harness.state_mut().canvas.options.interaction.snapping = true;
    harness.step();
    assert!(has_marker(&harness));
    harness.state_mut().canvas.options.grid_snap = GridSnapMode::Disabled;
    harness.step();
    assert!(!has_marker(&harness));
    harness.state_mut().canvas.options.grid_snap = GridSnapMode::Fixed;
    harness.step();
    assert!(has_marker(&harness));
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!has_marker(&harness));
    assert!(
        harness
            .state()
            .events_seen
            .iter()
            .any(|event| matches!(event, CanvasEvent::NodeDragCancelled { .. }))
    );
    assert!(
        !harness
            .state()
            .events_seen
            .iter()
            .any(|event| matches!(event, CanvasEvent::NodeDragFinished { .. }))
    );
}

#[test]
fn smooth_zoom_updates_adaptive_step_and_preserves_fixed_step_during_drag() {
    for mode in [GridSnapMode::Adaptive, GridSnapMode::Fixed] {
        let mut harness = harness(state(mode, 2.0));
        begin(&mut harness);
        let raw = pos2(21.0, 20.0);
        move_to(&mut harness, raw);
        assert!(last_position(&harness).distance(pos2(20.0, 20.0)) < 0.001);
        for (delta, captured) in [(-400.0, mode == GridSnapMode::Fixed), (400.0, true)] {
            harness.event(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, delta),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            });
            harness.run();
            assert_eq!(has_marker(&harness), captured);
            let expected = if captured { pos2(20.0, 20.0) } else { raw };
            assert!(last_position(&harness).distance(expected) < 0.001);
        }
        assert_eq!(harness.state().canvas.stats().snap_rebuilds, 0);
    }
}

#[test]
fn grid_competes_with_vertex_guides_without_sequential_snapping() {
    for (raw, expected, grid_marker) in [
        (pos2(1.0, 0.0), Pos2::ZERO, true),
        (pos2(3.0, 0.0), pos2(4.0, 0.0), false),
        (pos2(2.0, 0.0), pos2(4.0, 0.0), false),
    ] {
        let mut state = state(GridSnapMode::Fixed, 1.0);
        state.graph.nodes[1] = pos2(4.0, 100.0);
        state.canvas.options.snapping.kinds = SnapKinds::VERTICAL;
        let mut harness = harness(state);
        begin(&mut harness);
        move_to(&mut harness, raw);
        assert!(last_position(&harness).distance(expected) < 0.001);
        assert_eq!(has_marker(&harness), grid_marker);
    }
}

#[test]
fn group_drag_snaps_the_leader_preserves_offsets_and_cancel_restores_the_group() {
    let mut state = state(GridSnapMode::Fixed, 1.0);
    state.canvas.select_all(&state.graph);
    let before = [state.graph.nodes[0], state.graph.nodes[1]];
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            move |ui, state: &mut State| {
                State::show(state, ui);
                for event in state.canvas.events() {
                    match *event {
                        CanvasEvent::NodeDragged { position, .. }
                        | CanvasEvent::NodeDragFinished { position, .. } => {
                            let delta = position - before[0];
                            for (point, origin) in state.graph.nodes.iter_mut().zip(before) {
                                *point = origin + delta;
                            }
                            state.graph.revision = state.graph.revision.next_geometry();
                        }
                        CanvasEvent::NodeDragCancelled { .. } => {
                            state.graph.nodes.copy_from_slice(&before);
                            state.graph.revision = state.graph.revision.next_geometry();
                        }
                        _ => {}
                    }
                }
            },
            state,
        );
    begin(&mut harness);
    move_to(&mut harness, pos2(21.0, 22.0));
    harness.run();
    assert!(harness.state().graph.nodes[0].distance(pos2(20.0, 20.0)) < 0.001);
    assert_eq!(
        harness.state().graph.nodes[1] - harness.state().graph.nodes[0],
        before[1] - before[0]
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().graph.nodes, before);
    assert!(!has_marker(&harness));
}
