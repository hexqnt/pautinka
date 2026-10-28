use std::cell::RefCell;

use egui::{Event, Key, Modifiers, PointerButton, Stroke, pos2};
use pautinka_egui::{
    CanvasEvent, ChangeCoverage, ElementId, GeometryChanges, GraphId, GraphRevision, Group,
    GroupFrame, GroupId, GroupInteraction, GroupState, GroupVisual, InteractionOptions, NodeId,
    PreparedGraph, RenderContext, Renderer,
};

use super::*;

struct Frame {
    id: GroupId,
    geometry: GroupFrame,
    state: Option<GroupState>,
    interaction: GroupInteraction,
    label: &'static str,
}

struct GroupGraph {
    id: GraphId,
    revision: GraphRevision,
    changes: GeometryChanges,
    frames: Vec<Frame>,
    content: bool,
    positions: [Pos2; 2],
    path: EdgePath,
}

impl Default for GroupGraph {
    fn default() -> Self {
        let positions = [pos2(-80.0, 30.0), pos2(80.0, 30.0)];
        Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default(),
            changes: GeometryChanges::default(),
            frames: [
                (0, "Plant", pos2(-180.0, -120.0), pos2(180.0, 120.0)),
                (1, "Cabinet", pos2(-130.0, -50.0), pos2(130.0, 80.0)),
            ]
            .map(|(id, label, min, max)| Frame {
                id: GroupId::new(id),
                label,
                geometry: GroupFrame::new(Rect::from_min_max(min, max), 24.0).unwrap(),
                state: Some(GroupState::Expanded),
                interaction: GroupInteraction::default(),
            })
            .into(),
            content: true,
            positions,
            path: EdgePath::try_line(positions[0], positions[1]).unwrap(),
        }
    }
}

impl Graph for GroupGraph {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn node_count(&self) -> usize {
        if self.content { 2 } else { 0 }
    }
    fn edge_count(&self) -> usize {
        usize::from(self.content)
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(
            NodeId::new(index as u64),
            self.positions[index],
            ["Pump", "Valve"][index],
        )
    }
    fn edge(&self, _: usize) -> Edge<'_> {
        Edge::new(pautinka_egui::EdgeId::new(0), &self.path)
    }
    fn bounds(&self) -> Rect {
        self.frames.iter().fold(Rect::ZERO, |bounds, frame| {
            bounds.union(frame.geometry.bounds())
        })
    }
    fn group_count(&self) -> usize {
        self.frames.len()
    }
    fn group(&self, index: usize) -> Group<'_> {
        let frame = &self.frames[index];
        let mut group = Group::new(frame.id, frame.geometry, frame.label);
        group.state = frame.state;
        group
    }
    fn group_interaction(&self, index: usize) -> GroupInteraction {
        self.frames[index].interaction
    }
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        self.changes.visit(since, self.revision, visit)
    }
}

#[derive(Default)]
struct GroupHarness {
    graph: GroupGraph,
    canvas: Canvas,
    events: Vec<CanvasEvent>,
}

fn harness(state: GroupHarness) -> Harness<'static, GroupHarness> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, state: &mut GroupHarness| {
                state.canvas.show(ui, &state.graph);
                state.events.extend_from_slice(state.canvas.events());
            },
            state,
        )
}

fn screen_position(harness: &Harness<'_, GroupHarness>, point: Pos2) -> Pos2 {
    harness
        .state()
        .canvas
        .camera
        .to_screen(point, harness.get_by_label("Graph canvas").rect())
}

fn click_at_world(harness: &mut Harness<'_, GroupHarness>, point: Pos2) {
    let point = screen_position(harness, point);
    pointer_gesture(
        harness,
        point,
        point,
        PointerButton::Primary,
        Modifiers::NONE,
    );
}

#[test]
fn frames_pick_only_headers_and_borders_with_content_above_them() {
    for prepared in [false, true] {
        let mut state = GroupHarness::default();
        if prepared {
            let prepared = PreparedGraph::new(&state.graph).unwrap();
            assert!(state.canvas.install_prepared(prepared, &state.graph));
        }
        for (point, expected) in [
            (pos2(0.0, -108.0), Some(Selection::Group(0))),
            (pos2(0.0, -38.0), Some(Selection::Group(1))),
            (pos2(130.0, 20.0), Some(Selection::Group(1))),
            (pos2(0.0, 0.0), None),
            (pos2(-80.0, 30.0), Some(Selection::Node(0))),
            (pos2(0.0, 30.0), Some(Selection::Edge(0))),
        ] {
            assert_eq!(
                state.canvas.pick(point, &state.graph),
                expected,
                "prepared={prepared}, point={point:?}"
            );
        }
        let mut harness = harness(state);
        click_at_world(&mut harness, pos2(0.0, -38.0));
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Group(1)));
        click_at_world(&mut harness, pos2(130.0, 20.0));
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Group(1)));
        click_label(&mut harness, "Pump");
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));
        click_at_world(&mut harness, pos2(0.0, 0.0));
        assert_eq!(harness.state().canvas.selected(), None);
    }
}

#[test]
fn collapse_is_only_a_request_and_hidden_content_is_owned_by_the_application() {
    let mut harness = harness(GroupHarness::default());
    let toggle = screen_position(&harness, pos2(-125.0, -45.0));
    pointer_gesture(
        &mut harness,
        toggle,
        toggle,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert!(
        harness
            .state()
            .events
            .contains(&CanvasEvent::GroupCollapseRequested {
                group: GroupId::new(1),
                collapsed: true
            })
    );
    assert_eq!(
        harness.state().graph.frames[1].state,
        Some(GroupState::Expanded)
    );
    assert!(harness.query_by_label("Pump").is_some());
    assert_eq!(harness.state().canvas.selected(), None);
    // Владелец может поменять только оформление и сохранить видимое содержимое.
    let state = harness.state_mut();
    state.graph.frames[1].state = Some(GroupState::Collapsed);
    state.graph.revision = state.graph.revision.next_appearance();
    harness.run();
    assert!(harness.query_by_label("Pump").is_some());
    click_at_world(&mut harness, pos2(0.0, 0.0));
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Group(1)));
    // Затем приложение само строит представление без внутренних узлов и рёбер.
    let state = harness.state_mut();
    state.graph.content = false;
    state.graph.revision = state.graph.revision.next_topology();
    harness.run();
    assert!(harness.query_by_label("Pump").is_none());
    assert_eq!(
        harness.state().canvas.selected_id(),
        Some(ElementId::Group(GroupId::new(1)))
    );
}

#[test]
fn group_drag_emits_total_world_delta_and_never_moves_the_model() {
    let mut harness = harness(GroupHarness::default());
    let frame = harness.state().graph.frames[1].geometry;
    let positions = harness.state().graph.positions;
    let from = screen_position(&harness, pos2(0.0, -38.0));
    let delta = Vec2::new(30.0, 20.0);
    let to = from + delta * harness.state().canvas.camera.zoom;
    let camera = harness.state().canvas.camera;
    pointer_gesture(
        &mut harness,
        from,
        to,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let events = &harness.state().events;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, CanvasEvent::GroupDragStarted { .. }))
            .count(),
        1
    );
    assert!(events.iter().any(|event| matches!(event, CanvasEvent::GroupDragged { group, delta: actual } if *group == GroupId::new(1) && (*actual - delta).length() < 0.001)));
    assert!(events.iter().any(|event| matches!(event, CanvasEvent::GroupDragFinished { group, delta: actual } if *group == GroupId::new(1) && (*actual - delta).length() < 0.001)));
    assert!(!events.iter().any(|event| matches!(
        event,
        CanvasEvent::NodeDragStarted { .. } | CanvasEvent::GroupDragCancelled { .. }
    )));
    assert_eq!(harness.state().canvas.camera, camera);
    assert_eq!(harness.state().graph.frames[1].geometry, frame);
    assert_eq!(harness.state().graph.positions, positions);
}

#[test]
fn group_drag_keeps_its_origin_when_the_owner_moves_and_reorders_frames() {
    let mut harness = harness(GroupHarness::default());
    let from = screen_position(&harness, pos2(0.0, -38.0));
    let zoom = harness.state().canvas.camera.zoom;
    harness.hover_at(from);
    harness.event(pointer_button(
        from,
        PointerButton::Primary,
        true,
        Modifiers::NONE,
    ));
    harness.step();
    harness.event(Event::PointerMoved(from + Vec2::new(20.0, 10.0) * zoom));
    harness.step();
    let state = harness.state_mut();
    let geometry = state.graph.frames[1].geometry;
    state.graph.frames[1].geometry = GroupFrame::new(
        geometry.bounds().translate(Vec2::new(20.0, 10.0)),
        geometry.header().height(),
    )
    .unwrap();
    state.graph.frames.swap(0, 1);
    state.graph.revision = state.graph.revision.next_topology();
    harness.step();
    let to = from + Vec2::new(50.0, 30.0) * zoom;
    harness.event(Event::PointerMoved(to));
    harness.step();
    harness.event(pointer_button(
        to,
        PointerButton::Primary,
        false,
        Modifiers::NONE,
    ));
    harness.run();
    assert!(harness.state().events.iter().any(|event| matches!(event,
        CanvasEvent::GroupDragFinished { group, delta }
            if *group == GroupId::new(1) && (*delta - Vec2::new(50.0, 30.0)).length() < 0.001
    )));
    assert!(
        !harness
            .state()
            .events
            .iter()
            .any(|event| matches!(event, CanvasEvent::GroupDragCancelled { .. }))
    );
}

#[test]
fn expanded_interiors_allow_rectangle_selection_and_background_pan() {
    let mut harness = harness(GroupHarness::default());
    let from = screen_position(&harness, pos2(-120.0, -20.0));
    let to = screen_position(&harness, pos2(120.0, 65.0));
    let camera = harness.state().canvas.camera;
    pointer_gesture(
        &mut harness,
        from,
        to,
        PointerButton::Primary,
        Modifiers::SHIFT,
    );
    assert_eq!(harness.state().canvas.selected_elements().count(), 3);
    assert!(
        !harness
            .state()
            .canvas
            .selected_elements()
            .any(|id| matches!(id, ElementId::Group(_)))
    );
    assert_eq!(harness.state().canvas.camera, camera);
    let from = screen_position(&harness, pos2(0.0, 0.0));
    pointer_gesture(
        &mut harness,
        from,
        from + Vec2::new(25.0, 15.0),
        PointerButton::Primary,
        Modifiers::NONE,
    );
    assert_ne!(harness.state().canvas.camera.center, camera.center);
    assert!(
        !harness
            .state()
            .events
            .iter()
            .any(|event| matches!(event, CanvasEvent::GroupDragStarted { .. }))
    );
}

#[test]
fn group_gestures_cancel_on_escape_focus_loss_removal_and_pinning() {
    #[derive(Clone, Copy, Debug)]
    enum Ending {
        Escape,
        FocusLost,
        Removed,
        Pinned,
        Disabled,
        PointerGone,
    }
    for ending in [
        Ending::Escape,
        Ending::FocusLost,
        Ending::Removed,
        Ending::Pinned,
        Ending::Disabled,
        Ending::PointerGone,
    ] {
        let mut harness = harness(GroupHarness::default());
        let from = screen_position(&harness, pos2(0.0, -38.0));
        let to = from + Vec2::new(40.0, 20.0);
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
        assert!(
            harness
                .state()
                .events
                .contains(&CanvasEvent::GroupDragStarted {
                    group: GroupId::new(1)
                })
        );
        match ending {
            Ending::Escape => harness.key_press(Key::Escape),
            Ending::FocusLost => harness.input_mut().focused = false,
            Ending::Removed => {
                let state = harness.state_mut();
                state.graph.frames.pop();
                state.graph.revision = state.graph.revision.next_topology();
            }
            Ending::Pinned => harness.state_mut().graph.frames[1].interaction.draggable = false,
            Ending::Disabled => harness.state_mut().canvas.options.interaction.group_drag = false,
            Ending::PointerGone => harness.event(Event::PointerGone),
        }
        harness.step();
        harness.step();
        harness.input_mut().focused = true;
        harness.event(pointer_button(
            to,
            PointerButton::Primary,
            false,
            Modifiers::NONE,
        ));
        harness.run();
        assert_eq!(harness.state().events.iter().filter(|event| matches!(event, CanvasEvent::GroupDragCancelled { group } if *group == GroupId::new(1))).count(), 1, "ending={ending:?}");
        assert!(
            !harness
                .state()
                .events
                .iter()
                .any(|event| matches!(event, CanvasEvent::GroupDragFinished { .. }))
        );
    }
}

#[test]
fn group_selection_context_focus_and_identity_survive_reordering() {
    let mut harness = harness(GroupHarness::default());
    click_label(&mut harness, "Cabinet");
    assert_eq!(
        harness.state().canvas.selected_id(),
        Some(ElementId::Group(GroupId::new(1)))
    );
    harness
        .get_by_label("Pump")
        .click_modifiers(Modifiers::CTRL);
    harness.run();
    assert_eq!(harness.state().canvas.selected_elements().count(), 2);
    let point = screen_position(&harness, pos2(0.0, -38.0));
    pointer_gesture(
        &mut harness,
        point,
        point,
        PointerButton::Secondary,
        Modifiers::NONE,
    );
    assert_eq!(
        harness.state().canvas.context_target(),
        Some(ElementId::Group(GroupId::new(1)))
    );
    let state = harness.state_mut();
    assert!(
        state
            .canvas
            .focus_id(ElementId::Group(GroupId::new(1)), &state.graph)
    );
    state.graph.frames.swap(0, 1);
    state.graph.revision = state.graph.revision.next_topology();
    harness.run();
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Group(0)));
    let bounds = harness.state().graph.frames[0].geometry.bounds();
    let center = screen_position(&harness, bounds.center());
    assert!(center.distance(harness.get_by_label("Graph canvas").rect().center()) < 0.001);
    let state = harness.state_mut();
    state.graph.frames.remove(0);
    state.graph.revision = state.graph.revision.next_topology();
    harness.run();
    assert_eq!(harness.state().canvas.selected(), None);
}

#[test]
fn read_only_and_disabled_policies_cover_group_actions() {
    for mode in 0..3 {
        let mut state = GroupHarness::default();
        state.canvas.options.interaction = match mode {
            0 => InteractionOptions::read_only(),
            1 => InteractionOptions::disabled(),
            _ => {
                state.graph.frames[1].interaction = GroupInteraction {
                    selectable: false,
                    draggable: false,
                };
                InteractionOptions {
                    group_toggle: false,
                    ..InteractionOptions::default()
                }
            }
        };
        let mut harness = harness(state);
        click_label(&mut harness, "Cabinet");
        assert_eq!(harness.state().canvas.selected().is_some(), mode == 0);
        let from = screen_position(&harness, pos2(0.0, -38.0));
        pointer_gesture(
            &mut harness,
            from,
            from + Vec2::new(50.0, 20.0),
            PointerButton::Primary,
            Modifiers::NONE,
        );
        let toggle = screen_position(&harness, pos2(-125.0, -45.0));
        pointer_gesture(
            &mut harness,
            toggle,
            toggle,
            PointerButton::Primary,
            Modifiers::NONE,
        );
        assert!(!harness.state().events.iter().any(|event| matches!(
            event,
            CanvasEvent::GroupDragStarted { .. } | CanvasEvent::GroupCollapseRequested { .. }
        )));
    }
}

#[test]
fn group_only_scene_supports_keyboard_and_accessibility_activation() {
    let mut state = GroupHarness::default();
    state.graph.content = false;
    let mut harness = harness(state);
    click_label(&mut harness, "Cabinet");
    harness.key_press(Key::ArrowUp);
    harness.run();
    assert_eq!(
        harness.state().canvas.selected_id(),
        Some(ElementId::Group(GroupId::new(0)))
    );
    harness.get_by_label("Cabinet").click_accesskit();
    harness.run();
    assert_eq!(
        harness.state().canvas.selected_id(),
        Some(ElementId::Group(GroupId::new(1)))
    );
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.run();
    assert_eq!(harness.state().canvas.selected_elements().count(), 2);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().canvas.selected(), None);
}

#[test]
fn group_accessibility_actions_work_with_the_pointer_over_a_node() {
    let mut harness = harness(GroupHarness::default());
    harness.get_by_label("Pump").hover();
    harness.run();
    harness.get_by_label("Cabinet").click_accesskit();
    harness.run();
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Group(1)));
    let expected = harness.state().graph.frames[1].geometry.header().center();
    assert!(harness.state().events.iter().any(|event| matches!(event,
        CanvasEvent::Clicked { target: Some(ElementId::Group(id)), position }
            if *id == GroupId::new(1) && *position == expected
    )));
    harness
        .get_all_by_label("Collapse group")
        .nth(1)
        .unwrap()
        .click_accesskit();
    harness.run();
    assert!(
        harness
            .state()
            .events
            .contains(&CanvasEvent::GroupCollapseRequested {
                group: GroupId::new(1),
                collapsed: true
            })
    );
}

#[test]
fn group_geometry_updates_use_the_existing_incremental_index() {
    let mut state = GroupHarness::default();
    state.canvas.prepare(&state.graph).unwrap();
    let before = state.canvas.stats();
    let geometry = state.graph.frames[1].geometry;
    let moved = GroupFrame::new(
        geometry.bounds().translate(Vec2::new(500.0, 0.0)),
        geometry.header().height(),
    )
    .unwrap();
    state.graph.frames[1].geometry = moved;
    let previous = state.graph.revision;
    state.graph.revision = previous.next_geometry();
    state
        .graph
        .changes
        .record(previous, state.graph.revision, [Selection::Group(1)]);
    state.canvas.prepare(&state.graph).unwrap();
    assert_eq!(state.canvas.stats().rebuilds, before.rebuilds);
    assert_eq!(
        state.canvas.stats().updated_entries,
        before.updated_entries + 1
    );
    assert_eq!(
        state.canvas.pick(moved.header().center(), &state.graph),
        Some(Selection::Group(1))
    );
    assert_eq!(
        state.canvas.pick(geometry.header().center(), &state.graph),
        None
    );
}

#[test]
fn custom_renderer_receives_groups_in_order_before_nodes() {
    #[derive(Default)]
    struct Recorder(RefCell<Vec<&'static str>>);
    impl Renderer for Recorder {
        fn paint_group(&self, context: &RenderContext<'_>, group: Group<'_>, visual: &GroupVisual) {
            self.0.borrow_mut().push(if group.id == GroupId::new(0) {
                "outer"
            } else {
                "inner"
            });
            assert_eq!(visual.style.stroke, Stroke::new(1.0, group.color));
            pautinka_egui::paint_default_group(context, group, visual);
        }
        fn paint_node(
            &self,
            context: &RenderContext<'_>,
            node: Node<'_>,
            visual: &pautinka_egui::NodeVisual,
        ) {
            self.0.borrow_mut().push("node");
            pautinka_egui::paint_default_node(context, node, visual);
        }
    }
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, state: &mut (GroupHarness, Recorder)| {
                state.1.0.borrow_mut().clear();
                state.0.canvas.show_with(ui, &state.0.graph, &state.1);
            },
            (GroupHarness::default(), Recorder::default()),
        );
    harness.run();
    assert_eq!(
        *harness.state().1.0.borrow(),
        ["outer", "inner", "node", "node"]
    );
}
