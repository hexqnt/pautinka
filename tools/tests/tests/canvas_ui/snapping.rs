use super::*;

#[test]
fn alignment_drag_snaps_axes_at_each_zoom_and_clears_guides_on_release() {
    use egui::{Event, Modifiers, PointerButton, Shape};
    use pautinka_egui::{CanvasEvent, NodeId, geometry::Camera};

    for zoom in [0.5, 2.0] {
        for (enabled, raw, expected, axes) in [
            (true, pos2(2.0, 98.0), pos2(0.0, 100.0), 2),
            (true, pos2(-40.0, 82.0), pos2(-40.0, 80.0), 1),
            (true, pos2(2.0, 40.0), pos2(0.0, 40.0), 1),
            (true, pos2(-40.0, 40.0), pos2(-40.0, 40.0), 0),
            (true, pos2(-70.0, 2.0), pos2(-70.0, 2.0), 0),
            (false, pos2(2.0, 98.0), pos2(2.0, 98.0), 0),
        ] {
            let mut state = State::default();
            state.graph.nodes[1] = pos2(0.0, 80.0);
            state.graph.nodes.push(pos2(90.0, 100.0));
            state.graph.node_ids.push(NodeId::new(2));
            state.canvas.options.interaction.snapping = enabled;
            state.canvas.options.camera.limit_zoom_out_to_fit = false;
            state.canvas.set_camera(Camera {
                center: Pos2::ZERO,
                zoom,
                fit: false,
            });
            let mut harness = canvas_harness(state);
            let viewport = harness.get_by_label("Graph canvas").rect();
            // Захват вне центра проверяет сохранение смещения указателя при привязке.
            let offset = Vec2::new(2.0, 1.0);
            let from = harness
                .state()
                .canvas
                .camera
                .to_screen(harness.state().graph.nodes[0], viewport)
                + offset;
            let to = harness.state().canvas.camera.to_screen(raw, viewport) + offset;
            harness.hover_at(from);
            harness.event(pointer_button(
                from,
                PointerButton::Primary,
                true,
                Modifiers::NONE,
            ));
            harness.step();
            harness.event(Event::PointerMoved(to));
            harness.step();
            let dragged = harness
                .state()
                .events_seen
                .iter()
                .rev()
                .find_map(|event| match event {
                    CanvasEvent::NodeDragged { position, .. } => Some(*position),
                    _ => None,
                })
                .unwrap();
            assert!(
                dragged.distance(expected) < 0.001,
                "{dragged:?} != {expected:?}"
            );
            let guide_segments = |harness: &Harness<'_, State>| {
                harness.output().shapes.iter().filter(|shape| matches!(&shape.shape,
                    Shape::LineSegment { stroke, .. } if stroke.color == harness.state().canvas.style.highlight
                )).count()
            };
            assert_eq!(guide_segments(&harness) > 0, axes > 0);
            if axes == 2 {
                for horizontal in [true, false] {
                    let count = harness.output().shapes.iter().filter(|shape| matches!(&shape.shape,
                        Shape::LineSegment { points, stroke } if stroke.color == harness.state().canvas.style.highlight
                            && if horizontal { points[0].y == points[1].y } else { points[0].x == points[1].x }
                    )).count();
                    assert!(count > 2);
                }
            }
            harness.event(pointer_button(
                to,
                PointerButton::Primary,
                false,
                Modifiers::NONE,
            ));
            harness.run();
            assert_eq!(guide_segments(&harness), 0);
            let finished = harness
                .state()
                .events_seen
                .iter()
                .find_map(|event| match event {
                    CanvasEvent::NodeDragFinished { position, .. } => Some(*position),
                    _ => None,
                })
                .unwrap();
            assert!(finished.distance(expected) < 0.001);
        }
    }
}

#[test]
fn alignment_sources_exclude_only_nodes_moving_with_the_drag() {
    use pautinka_egui::{CanvasEvent, geometry::Camera};

    for (selection_enabled, pinned, expected_y) in
        [(false, false, 0.0), (true, true, 0.0), (true, false, 2.0)]
    {
        let mut state = State::default();
        state.graph.first_pinned = pinned;
        state.canvas.select(Some(Selection::Node(0)), &state.graph);
        if selection_enabled {
            state.canvas.select_all(&state.graph);
        }
        state.canvas.options.interaction.selection = selection_enabled;
        state.canvas.options.interaction.snapping = true;
        state.canvas.options.camera.limit_zoom_out_to_fit = false;
        state.canvas.set_camera(Camera {
            center: Pos2::ZERO,
            zoom: 1.0,
            fit: false,
        });
        let mut harness = canvas_harness(state);
        let viewport = harness.get_by_label("Graph canvas").rect();
        let camera = harness.state().canvas.camera;
        let from = camera.to_screen(harness.state().graph.nodes[1], viewport);
        let to = camera.to_screen(pos2(40.0, 2.0), viewport);
        pointer_gesture(
            &mut harness,
            from,
            to,
            egui::PointerButton::Primary,
            egui::Modifiers::NONE,
        );
        let position = harness
            .state()
            .events_seen
            .iter()
            .find_map(|event| match event {
                CanvasEvent::NodeDragFinished { position, .. } => Some(*position),
                _ => None,
            })
            .unwrap();
        assert!((position.y - expected_y).abs() < 0.001);
    }
}

#[test]
fn alignment_drag_holds_nearby_axes_and_releases_or_invalidates_them() {
    use egui::{Event, Modifiers, PointerButton};
    use pautinka_egui::{CanvasEvent, NodeId, geometry::Camera};

    for zoom in [1.0, 2.0] {
        let mut state = State::default();
        state.graph.nodes = vec![
            pos2(-100.0, 0.0) / zoom,
            pos2(0.0, 80.0) / zoom,
            pos2(8.0, 100.0) / zoom,
        ];
        state.graph.node_ids.push(NodeId::new(2));
        state.canvas.options.interaction.snapping = true;
        state.canvas.options.camera.limit_zoom_out_to_fit = false;
        state.canvas.set_camera(Camera {
            center: Pos2::ZERO,
            zoom,
            fit: false,
        });
        let mut harness = canvas_harness(state);
        let viewport = harness.get_by_label("Graph canvas").rect();
        let camera = harness.state().canvas.camera;
        let from = camera.to_screen(harness.state().graph.nodes[0], viewport);
        harness.hover_at(from);
        harness.event(pointer_button(
            from,
            PointerButton::Primary,
            true,
            Modifiers::NONE,
        ));
        harness.step();
        for (raw, expected) in [
            (pos2(1.0, 82.0), pos2(0.0, 80.0)),
            (pos2(7.0, 87.0), pos2(0.0, 80.0)),
            (pos2(10.0, 87.0), pos2(8.0, 80.0)),
            (pos2(18.0, 95.0), pos2(18.0, 100.0)),
            (pos2(7.0, 82.0), pos2(8.0, 80.0)),
        ] {
            harness.event(Event::PointerMoved(camera.to_screen(raw / zoom, viewport)));
            harness.step();
            let position = harness
                .state()
                .events_seen
                .iter()
                .rev()
                .find_map(|event| match event {
                    CanvasEvent::NodeDragged { position, .. } => Some(*position),
                    _ => None,
                })
                .unwrap();
            assert!(
                position.distance(expected / zoom) < 0.001,
                "{position:?} != {expected:?} at zoom {zoom}"
            );
        }
        assert_eq!(harness.state().canvas.stats().snap_rebuilds, 1);
        // Удаление прежней оси должно немедленно отменять удержание.
        harness.state_mut().graph.nodes[2] = pos2(30.0, 100.0) / zoom;
        harness.state_mut().graph.revision = harness.state().graph.revision.next_geometry();
        harness.step();
        assert!(harness.state().canvas.events().iter().any(|event| matches!(event,
            CanvasEvent::NodeDragged { position, .. } if position.distance(pos2(7.0, 80.0) / zoom) < 0.001
        )));
        harness.event(Event::PointerMoved(
            camera.to_screen(pos2(32.0, 95.0) / zoom, viewport),
        ));
        harness.step();
        assert_eq!(harness.state().canvas.stats().snap_rebuilds, 2);
        assert!(harness.state().canvas.events().iter().any(|event| matches!(event,
            CanvasEvent::NodeDragged { position, .. } if position.distance(pos2(30.0, 100.0) / zoom) < 0.001
        )));
        harness.state_mut().canvas.options.interaction.snapping = false;
        harness.step();
        assert!(harness.state().canvas.events().iter().any(|event| matches!(event,
            CanvasEvent::NodeDragged { position, .. } if position.distance(pos2(32.0, 95.0) / zoom) < 0.001
        )));
        harness.key_press(egui::Key::Escape);
        harness.step();
        assert!(
            harness
                .state()
                .events_seen
                .iter()
                .any(|event| matches!(event, CanvasEvent::NodeDragCancelled { .. }))
        );
    }
}

#[test]
fn diagonal_drag_uses_selected_modes_and_clips_guides_without_changing_their_slope() {
    use egui::{Event, Modifiers, PointerButton, Shape};
    use pautinka_egui::{CanvasEvent, SnapKind, SnapKinds, geometry::Camera};

    for zoom in [1.0, 2.0] {
        for (kind, source, raw, expected) in [
            (
                SnapKind::DiagonalUp,
                pos2(0.0, 80.0),
                pos2(40.0, 42.0),
                pos2(39.0, 41.0),
            ),
            (
                SnapKind::DiagonalDown,
                pos2(0.0, 80.0),
                pos2(40.0, 118.0),
                pos2(39.0, 119.0),
            ),
            (
                SnapKind::DiagonalUp,
                pos2(800.0, -720.0),
                pos2(40.0, 42.0),
                pos2(39.0, 41.0),
            ),
        ] {
            let mut state = State::default();
            state.graph.nodes = vec![pos2(-100.0, 0.0) / zoom, source / zoom];
            state.canvas.options.interaction.snapping = true;
            state.canvas.options.snapping.kinds = kind.flag();
            state.canvas.options.camera.limit_zoom_out_to_fit = false;
            state.canvas.set_camera(Camera {
                center: Pos2::ZERO,
                zoom,
                fit: false,
            });
            let mut harness = canvas_harness(state);
            let viewport = harness.get_by_label("Graph canvas").rect();
            let camera = harness.state().canvas.camera;
            let from = camera.to_screen(harness.state().graph.nodes[0], viewport);
            let to = camera.to_screen(raw / zoom, viewport);
            harness.hover_at(from);
            harness.event(pointer_button(
                from,
                PointerButton::Primary,
                true,
                Modifiers::NONE,
            ));
            harness.step();
            harness.event(Event::PointerMoved(to));
            harness.step();
            assert!(harness.state().events_seen.iter().any(|event| matches!(event,
                CanvasEvent::NodeDragged { position, .. } if position.distance(expected / zoom) < 0.001
            )));
            let guides: Vec<_> = harness
                .output()
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    Shape::LineSegment { points, stroke }
                        if stroke.color == harness.state().canvas.style.highlight =>
                    {
                        Some(points)
                    }
                    _ => None,
                })
                .collect();
            assert!(guides.len() > 2);
            for [a, b] in guides {
                assert!(((*a - *b).x.abs() - (*a - *b).y.abs()).abs() < 0.001);
                assert!(viewport.expand(0.001).contains(*a));
                assert!(viewport.expand(0.001).contains(*b));
            }
            harness.state_mut().canvas.options.snapping.kinds = SnapKinds::VERTICAL;
            harness.step();
            assert!(harness.state().canvas.events().iter().any(|event| matches!(event,
                CanvasEvent::NodeDragged { position, .. } if position.distance(raw / zoom) < 0.001
            )));
            assert_eq!(harness.state().canvas.stats().snap_rebuilds, 2);
            harness.event(pointer_button(
                to,
                PointerButton::Primary,
                false,
                Modifiers::NONE,
            ));
            harness.run();
            assert!(harness.state().events_seen.iter().any(|event| matches!(event,
                CanvasEvent::NodeDragFinished { position, .. } if position.distance(raw / zoom) < 0.001
            )));
        }
    }
}
