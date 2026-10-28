use super::*;

#[test]
fn shift_rectangle_selects_the_graph_without_panning_and_keys_navigate_nodes() {
    let mut harness = canvas_harness(State::default());
    let viewport = harness.get_by_label("Graph canvas").rect();
    let camera = harness.state().canvas.camera;
    let from = viewport.min + Vec2::splat(30.0);
    let to = viewport.max - Vec2::splat(30.0);
    pointer_gesture(
        &mut harness,
        from,
        to,
        egui::PointerButton::Primary,
        egui::Modifiers::SHIFT,
    );
    assert_eq!(harness.state().canvas.selected_elements().count(), 4);
    assert_eq!(harness.state().canvas.camera.center, camera.center);
    assert_eq!(harness.state().canvas.camera.zoom, camera.zoom);

    click_label(&mut harness, "First");
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert_eq!(
        harness.state().canvas.selected_id(),
        Some(pautinka_egui::ElementId::Node(pautinka_egui::NodeId::new(
            1
        )))
    );
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    assert_eq!(harness.state().canvas.selected_elements().count(), 4);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().canvas.selected_elements().count(), 0);
}

#[test]
fn node_and_marquee_gestures_finish_once_or_cancel_without_committing() {
    use egui::{CursorIcon, Event, Modifiers, PointerButton};
    use pautinka_egui::CanvasEvent;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Ending {
        Release,
        Escape,
        FocusLost,
        PointerGone,
    }

    for marquee in [false, true] {
        for ending in [
            Ending::Release,
            Ending::Escape,
            Ending::FocusLost,
            Ending::PointerGone,
        ] {
            let mut harness = canvas_harness(State::default());
            let viewport = harness.get_by_label("Graph canvas").rect();
            let (from, to, modifiers) = if marquee {
                (
                    viewport.min + Vec2::splat(30.0),
                    viewport.max - Vec2::splat(30.0),
                    Modifiers::SHIFT,
                )
            } else {
                let from = harness.get_by_label("First").rect().center();
                (from, from + Vec2::new(40.0, 20.0), Modifiers::NONE)
            };
            let camera = harness.state().canvas.camera;
            harness.event(Event::ModifiersChanged(modifiers));
            harness.hover_at(from);
            harness.step();
            assert_eq!(
                harness.output().platform_output.cursor_icon,
                if marquee {
                    CursorIcon::Crosshair
                } else {
                    CursorIcon::Grab
                },
            );
            harness.event_modifiers(
                pointer_button(from, PointerButton::Primary, true, modifiers),
                modifiers,
            );
            harness.step();
            harness.event_modifiers(Event::PointerMoved(to), modifiers);
            harness.step();
            assert_eq!(
                harness.output().platform_output.cursor_icon,
                if marquee {
                    CursorIcon::Crosshair
                } else {
                    CursorIcon::Grabbing
                },
            );
            if marquee {
                assert_eq!(harness.state().canvas.selected_elements().count(), 0);
            } else {
                assert!(
                    harness
                        .state()
                        .events_seen
                        .iter()
                        .any(|event| matches!(event, CanvasEvent::NodeDragStarted { .. }))
                );
            }

            match ending {
                Ending::Release => harness.event_modifiers(
                    pointer_button(to, PointerButton::Primary, false, modifiers),
                    modifiers,
                ),
                Ending::Escape => harness.key_press(egui::Key::Escape),
                Ending::FocusLost => harness.input_mut().focused = false,
                Ending::PointerGone => harness.event(Event::PointerGone),
            }
            // PointerGone сохраняет interact_pos на последний кадр взаимодействия.
            harness.step();
            harness.step();
            harness.input_mut().focused = true;
            harness.event_modifiers(Event::PointerMoved(to), modifiers);
            harness.event_modifiers(
                pointer_button(to, PointerButton::Primary, false, modifiers),
                modifiers,
            );
            harness.run();

            let state = harness.state();
            let finished = state
                .events_seen
                .iter()
                .filter(|event| matches!(event, CanvasEvent::NodeDragFinished { .. }))
                .count();
            let cancelled = state
                .events_seen
                .iter()
                .filter(|event| matches!(event, CanvasEvent::NodeDragCancelled { .. }))
                .count();
            assert_eq!(
                finished,
                usize::from(!marquee && ending == Ending::Release),
                "marquee={marquee}, ending={ending:?}"
            );
            assert_eq!(
                cancelled,
                usize::from(!marquee && ending != Ending::Release),
                "marquee={marquee}, ending={ending:?}"
            );
            if marquee {
                assert_eq!(
                    state.canvas.selected_elements().count(),
                    if ending == Ending::Release { 4 } else { 0 },
                    "ending={ending:?}"
                );
                assert_eq!(
                    state.canvas.selection_revision(),
                    u64::from(ending == Ending::Release),
                    "ending={ending:?}"
                );
            }
            assert_eq!(state.canvas.camera, camera);
            assert_ne!(
                harness.output().platform_output.cursor_icon,
                CursorIcon::Grabbing
            );
        }
    }
}

#[test]
fn read_only_pinning_and_disabled_selection_are_independent() {
    use egui::CursorIcon;
    use pautinka_egui::{CanvasEvent, InteractionOptions};
    for (read_only, pinned) in [(true, false), (false, true), (false, false)] {
        let mut state = State::default();
        state.graph.first_pinned = pinned;
        if read_only {
            state.canvas.options.interaction = InteractionOptions::read_only();
        }
        let mut harness = canvas_harness(state);
        let point = harness.get_by_label("First").rect().center();
        harness.hover_at(point);
        harness.run();
        assert_eq!(
            harness.output().platform_output.cursor_icon,
            if read_only || pinned {
                CursorIcon::PointingHand
            } else {
                CursorIcon::Grab
            },
        );
        pointer_gesture(
            &mut harness,
            point,
            point + Vec2::new(40.0, 20.0),
            egui::PointerButton::Primary,
            egui::Modifiers::NONE,
        );
        let dragged = harness
            .state()
            .events_seen
            .iter()
            .any(|e| matches!(e, CanvasEvent::NodeDragged { .. }));
        assert_eq!(dragged, !read_only && !pinned);
        click_role_label(&mut harness, egui::accesskit::Role::Button, "First");
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));
    }
    let mut state = State::default();
    state.canvas.options.interaction.selection = false;
    state.canvas.options.interaction.node_drag = false;
    let mut harness = canvas_harness(state);
    click_label(&mut harness, "First");
    assert_eq!(harness.state().canvas.selected(), None);
    harness.state_mut().graph.first_selectable = false;
    harness.state_mut().canvas.options.interaction.selection = true;
    click_label(&mut harness, "First");
    assert_eq!(harness.state().canvas.selected(), None);
    harness.state_mut().canvas.options.interaction = InteractionOptions::disabled();
    harness.hover_at(harness.get_by_label("First").rect().center());
    harness.run();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::Default
    );
}

#[test]
fn pan_button_modifiers_and_zoom_can_be_configured() {
    use egui::{CursorIcon, Event, Modifiers, PointerButton};
    use pautinka_egui::ModifierKey;
    let mut state = State::default();
    state.canvas.options.interaction.pan_button = PointerButton::Middle;
    state.canvas.options.interaction.pan_modifier = ModifierKey::Alt;
    state.canvas.options.interaction.zoom = false;
    let mut harness = canvas_harness(state);
    let point = harness.get_by_label("Graph canvas").rect().min + Vec2::splat(40.0);
    let original = harness.state().canvas.camera;
    pointer_gesture(
        &mut harness,
        point,
        point + Vec2::new(50.0, 0.0),
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.camera, original);
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.hover_at(point);
    harness.step();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::Grab
    );
    harness.event(pointer_button(
        point,
        PointerButton::Middle,
        true,
        Modifiers::ALT,
    ));
    harness.step();
    let to = point + Vec2::new(50.0, 0.0);
    harness.event(Event::PointerMoved(to));
    harness.step();
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::Grabbing
    );
    harness.event(pointer_button(
        to,
        PointerButton::Middle,
        false,
        Modifiers::ALT,
    ));
    harness.run();
    assert_ne!(harness.state().canvas.camera.center, original.center);
    let zoom = harness.state().canvas.camera.zoom;
    harness.event(egui::Event::Zoom(2.0));
    harness.run();
    assert_eq!(harness.state().canvas.camera.zoom, zoom);
}

#[test]
fn custom_selection_and_drag_modifiers_are_used_consistently() {
    use pautinka_egui::{CanvasEvent, ModifierKey};
    let mut state = State::default();
    state.canvas.options.interaction.toggle_modifier = ModifierKey::Alt;
    state.canvas.options.interaction.node_drag_modifier = ModifierKey::Control;
    let mut harness = canvas_harness(state);
    click_label(&mut harness, "First");
    harness
        .get_by_label("Second")
        .click_modifiers(egui::Modifiers::ALT);
    harness.run();
    assert_eq!(harness.state().canvas.selected_elements().count(), 2);
    let point = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "First")
        .rect()
        .center();
    pointer_gesture(
        &mut harness,
        point,
        point + Vec2::new(30.0, 10.0),
        egui::PointerButton::Primary,
        egui::Modifiers::CTRL,
    );
    assert!(
        harness
            .state()
            .events_seen
            .iter()
            .any(|event| matches!(event, CanvasEvent::NodeDragStarted { .. }))
    );
    harness.state_mut().canvas.options.interaction = pautinka_egui::InteractionOptions::disabled();
    let camera = harness.state().canvas.camera;
    let selection = harness.state().canvas.selected_id();
    click_role_label(&mut harness, egui::accesskit::Role::Button, "Second");
    assert_eq!(harness.state().canvas.selected_id(), selection);
    assert_eq!(harness.state().canvas.camera, camera);
}

#[test]
fn escape_cancels_marquee_without_clearing_existing_selection() {
    use egui::{Event, Modifiers, PointerButton};

    for modifiers_on_escape in [Modifiers::NONE, Modifiers::SHIFT] {
        let mut state = State::default();
        state.canvas.select(Some(Selection::Node(0)), &state.graph);
        let mut harness = canvas_harness(state);
        harness.state_mut().events_seen.clear();
        let viewport = harness.get_by_label("Graph canvas").rect();
        let from = viewport.min + Vec2::splat(30.0);
        harness.hover_at(from);
        harness.event_modifiers(
            pointer_button(from, PointerButton::Primary, true, Modifiers::SHIFT),
            Modifiers::SHIFT,
        );
        harness.step();
        let to = viewport.max - Vec2::splat(30.0);
        harness.event_modifiers(Event::PointerMoved(to), Modifiers::SHIFT);
        harness.step();
        harness.key_press_modifiers(modifiers_on_escape, egui::Key::Escape);
        harness.step();
        harness.event(pointer_button(
            to,
            PointerButton::Primary,
            false,
            Modifiers::NONE,
        ));
        harness.run();
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));
        assert_eq!(harness.state().canvas.selected_elements().count(), 1);
        assert!(
            !harness
                .state()
                .events_seen
                .iter()
                .any(|event| matches!(event, pautinka_egui::CanvasEvent::SelectionChanged(None)))
        );
    }
}
