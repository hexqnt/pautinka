use super::*;
use pautinka_egui::{CanvasEvent, NavigatorCorner};

#[test]
fn navigator_is_optional_and_follows_the_selected_corner() {
    let mut harness = canvas_harness(State::default());
    assert!(harness.query_by_label("Navigator").is_none());
    harness.state_mut().canvas.options.navigator.enabled = true;
    for corner in [
        NavigatorCorner::TopLeft,
        NavigatorCorner::TopRight,
        NavigatorCorner::BottomLeft,
        NavigatorCorner::BottomRight,
    ] {
        harness.state_mut().canvas.options.navigator.corner = corner;
        harness.run();
        let viewport = harness.get_by_label("Graph canvas").rect();
        let panel = harness.get_by_label("Navigator").rect();
        assert!(viewport.contains_rect(panel));
        let expected = match corner {
            NavigatorCorner::TopLeft => viewport.left_top() + Vec2::splat(12.0),
            NavigatorCorner::TopRight => {
                viewport.right_top() + Vec2::new(-panel.width() - 12.0, 12.0)
            }
            NavigatorCorner::BottomLeft => {
                viewport.left_bottom() + Vec2::new(12.0, -panel.height() - 12.0)
            }
            NavigatorCorner::BottomRight => {
                viewport.right_bottom() - panel.size() - Vec2::splat(12.0)
            }
        };
        assert_eq!(panel.min, expected);
    }
    harness.state_mut().canvas.options.navigator.enabled = false;
    harness.run();
    assert!(harness.query_by_label("Navigator").is_none());
}

#[test]
fn navigator_click_and_drag_pan_without_selecting_or_editing_graph() {
    let mut state = State::default();
    state.canvas.options.navigator.enabled = true;
    state.canvas.select(Some(Selection::Node(0)), &state.graph);
    let mut harness = canvas_harness(state);
    harness.state_mut().canvas.camera.zoom *= 4.0;
    let initial = harness.state().canvas.camera;
    let panel = harness.get_by_label("Navigator").rect();
    let from = panel.center() - Vec2::new(50.0, 0.0);
    let to = panel.center() + Vec2::new(50.0, 0.0);
    harness.state_mut().events_seen.clear();
    pointer_gesture(
        &mut harness,
        from,
        from,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    let left = harness.state().canvas.camera;
    assert!(left.center.x < initial.center.x);
    assert_eq!(left.zoom, initial.zoom);
    pointer_gesture(
        &mut harness,
        from,
        to,
        egui::PointerButton::Primary,
        egui::Modifiers::SHIFT,
    );
    let state = harness.state();
    assert!(state.canvas.camera.center.x > initial.center.x);
    assert_eq!(state.canvas.camera.zoom, initial.zoom);
    assert_eq!(state.canvas.selected(), Some(Selection::Node(0)));
    assert!(!state.events_seen.iter().any(|event| matches!(
        event,
        CanvasEvent::Clicked { .. }
            | CanvasEvent::Activated { .. }
            | CanvasEvent::NodeDragStarted { .. }
            | CanvasEvent::SelectionChanged(_)
    )));
    harness.state_mut().canvas.options.interaction.pan = false;
    let camera = harness.state().canvas.camera;
    pointer_gesture(
        &mut harness,
        to,
        from,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.camera, camera);
}

#[test]
fn navigator_captures_clicks_over_nodes_and_does_not_zoom_the_canvas() {
    let mut state = State::default();
    state.canvas.options.navigator.enabled = true;
    let mut harness = canvas_harness(state);
    let viewport = harness.get_by_label("Graph canvas").rect();
    let panel = harness.get_by_label("Navigator").rect();
    let camera = harness.state().canvas.camera;
    let node = harness.state().graph.nodes[0];
    harness.state_mut().canvas.camera.center =
        node - (panel.center() - viewport.center()) / camera.zoom;
    harness.run();
    harness.state_mut().events_seen.clear();
    pointer_gesture(
        &mut harness,
        panel.center(),
        panel.center(),
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.selected(), None);
    assert!(
        !harness
            .state()
            .events_seen
            .iter()
            .any(|event| matches!(event, CanvasEvent::Clicked { .. }))
    );
    let camera = harness.state().canvas.camera;
    harness.hover_at(panel.center());
    harness.event(egui::Event::Zoom(2.0));
    harness.run();
    assert_eq!(harness.state().canvas.camera, camera);
}

fn thumbnail_mesh(harness: &Harness<'_, State>) -> std::sync::Arc<egui::epaint::Mesh> {
    let clip = harness.get_by_label("Navigator").rect().shrink(8.0);
    harness
        .output()
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Mesh(mesh) if shape.clip_rect == clip => Some(std::sync::Arc::clone(mesh)),
            _ => None,
        })
        .expect("Navigator mesh")
}

#[test]
fn navigator_reuses_geometry_when_panning_and_refreshes_graph_changes() {
    let mut state = State::default();
    state.canvas.options.navigator.enabled = true;
    let mut harness = canvas_harness(state);
    harness.run();
    let original = thumbnail_mesh(&harness);
    assert!(original.is_valid());
    harness.state_mut().canvas.camera.center.x += 20.0;
    harness.state_mut().canvas.camera.zoom *= 2.0;
    harness.run();
    assert_eq!(original.texture_id, thumbnail_mesh(&harness).texture_id);
    assert_eq!(original.vertices.len(), 4);

    harness.state_mut().graph.node_color = Color32::RED;
    harness.state_mut().graph.revision = harness.state().graph.revision.next_appearance();
    harness.run();
    let recolored = thumbnail_mesh(&harness);
    assert_ne!(original.texture_id, recolored.texture_id);

    let state = harness.state_mut();
    state.canvas.set_show_loops(false, &state.graph);
    harness.run();
    let without_loop = thumbnail_mesh(&harness);
    assert_ne!(recolored.texture_id, without_loop.texture_id);

    harness.state_mut().canvas.options.navigator.corner = NavigatorCorner::BottomLeft;
    harness.run();
    let moved = thumbnail_mesh(&harness);
    assert_eq!(without_loop.texture_id, moved.texture_id);
    assert_ne!(without_loop.vertices[0].pos, moved.vertices[0].pos);

    let state = harness.state_mut();
    state.graph.nodes[0].y += 20.0;
    state.graph.revision = state.graph.revision.next_geometry();
    harness.run();
    assert_ne!(moved.texture_id, thumbnail_mesh(&harness).texture_id);

    let before_dpi = thumbnail_mesh(&harness).texture_id;
    harness.ctx.set_pixels_per_point(2.0);
    harness.run();
    assert_ne!(before_dpi, thumbnail_mesh(&harness).texture_id);

    let before_resize = thumbnail_mesh(&harness).texture_id;
    harness.set_size(Vec2::new(170.0, 160.0));
    harness.run();
    assert_ne!(before_resize, thumbnail_mesh(&harness).texture_id);
}

#[test]
fn navigator_keeps_curves_native_and_polyline_corners_exact() {
    use egui::Shape;

    for (polyline, rounded) in [(false, false), (true, false), (true, true)] {
        let mut state = State::default();
        state.canvas.options.navigator.enabled = true;
        state.graph.edges[0] = if polyline {
            EdgePath::try_polyline(vec![
                pos2(-100.0, 0.0),
                pos2(-100.0, 80.0),
                pos2(100.0, 80.0),
                pos2(100.0, 0.0),
            ])
            .unwrap()
        } else {
            EdgePath::try_quadratic([pos2(-100.0, 0.0), pos2(0.0, 120.0), pos2(100.0, 0.0)])
                .unwrap()
        };
        if rounded {
            state.graph.edges[0] = state.graph.edges[0]
                .clone()
                .with_corner_radius(pautinka_egui::CornerRadius::try_from(30.0).unwrap())
                .unwrap();
        }
        let mut harness = Harness::builder()
            .with_size(Vec2::new(800.0, 600.0))
            .build_ui_state(
                |ui, state: &mut State| {
                    state.graph.revision = state.graph.revision.next_geometry();
                    State::show(state, ui);
                },
                state,
            );
        harness.run_steps(3);
        if rounded {
            assert!(harness.state().graph.edges[0].points().len() > 4);
        }
        let clip = harness.get_by_label("Navigator").rect().shrink(8.0);
        let shapes: Vec<_> = harness
            .output()
            .shapes
            .iter()
            .filter(|shape| shape.clip_rect == clip)
            .map(|shape| &shape.shape)
            .collect();
        assert_eq!(
            shapes
                .iter()
                .filter(|shape| matches!(shape, Shape::CubicBezier(_)))
                .count(),
            1
        );
        assert_eq!(
            shapes
                .iter()
                .filter(|shape| matches!(shape, Shape::QuadraticBezier(_)))
                .count(),
            usize::from(!polyline)
        );
        let segments: Vec<_> = shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::LineSegment { points, .. } => Some(points),
                _ => None,
            })
            .collect();
        assert_eq!(segments.len(), if polyline { 3 } else { 0 });
        for pair in segments.windows(2) {
            assert_eq!(pair[0][1], pair[1][0]);
        }
        assert_eq!(
            shapes
                .iter()
                .filter(|shape| matches!(shape, Shape::Circle(_)))
                .count(),
            harness.state().graph.node_count()
        );
    }
}
