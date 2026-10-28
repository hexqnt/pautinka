#[path = "stress_ui/scenario.rs"]
mod scenario;

use egui::{
    Event, Modifiers, PointerButton, Vec2,
    accesskit::{Role, Toggled},
};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use pautinka_egui::{Graph, RoutingMode};
use pautinka_stress::{
    Demo, Scenario,
    scene::{Load, NodeKind},
};

fn harness(size: Vec2) -> Harness<'static, Demo> {
    Harness::builder()
        .with_size(size)
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(|ui, demo| demo.show(ui), Demo::default())
}

fn scenario_harness(scenario: Scenario) -> Harness<'static, Demo> {
    Harness::builder()
        .with_size(Vec2::new(1440.0, 960.0))
        .build_ui_state(|ui, demo| demo.show(ui), Demo::with_scenario(scenario))
}

fn click(harness: &mut Harness<'_, Demo>, label: &str) {
    harness.get_by_label(label).click();
    harness.run_steps(3);
}

fn pointer(position: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos: position,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn library_version_and_repository_link_stay_visible_in_the_header() {
    for width in [360.0, 1440.0] {
        let mut harness = harness(Vec2::new(width, 960.0));
        harness.run_steps(3);
        let version = harness
            .get_by_label(concat!("pautinka v", env!("CARGO_PKG_VERSION")))
            .rect();
        let link = harness.get_by_label("GitHub").rect();
        assert!(version.left() >= 0.0 && version.right() <= link.left());
        assert!(link.right() <= width);
        assert!(version.top() < link.bottom() && link.top() < version.bottom());
        assert!(link.bottom() < harness.get_by_label("Animated graph").rect().top());
    }
}

#[test]
fn animation_growth_and_restart_work_independently() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    for label in ["Animate", "Auto grow", "Auto fit"] {
        assert_eq!(
            harness
                .get_by_role_and_label(Role::Button, label)
                .accesskit_node()
                .toggled(),
            Some(Toggled::True)
        );
    }
    let before = harness.state().scene().node(0).position;
    harness.run_steps(4);
    assert_ne!(harness.state().scene().node(0).position, before);
    click(&mut harness, "Animate");
    assert_eq!(
        harness.get_by_label("Animate").accesskit_node().toggled(),
        Some(Toggled::False)
    );
    let frozen = harness.state().scene().node(0).position;
    harness.run_steps(605);
    assert_eq!(harness.state().scene().node_count(), 250);
    assert_eq!(harness.state().scene().node(0).position, frozen);
    click(&mut harness, "Auto grow");
    assert!(!harness.state().auto_grow());
    assert_eq!(
        harness.get_by_label("Auto grow").accesskit_node().toggled(),
        Some(Toggled::False)
    );
    harness.run_steps(10);
    assert!(harness.state().sample_count() > 0);
    click(&mut harness, "Restart");
    assert_eq!(harness.state().scene().node_count(), 100);
    assert!(!harness.state().running());
}

#[test]
fn slow_frames_keep_the_configured_growth_duration() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1440.0, 960.0))
        .with_step_dt(1.0)
        .build_ui_state(|ui, demo| demo.show(ui), Demo::default());
    harness.run_steps(11);
    assert_eq!(harness.state().scene().node_count(), 250);
}

#[test]
fn auto_fit_tracks_graph_size_and_can_be_disabled_and_reenabled() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    click(&mut harness, "Animate");
    click(&mut harness, "Auto grow");
    let initial = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
    click(&mut harness, "Load");
    harness
        .get_by_role_and_label(Role::Slider, "Nodes")
        .set_value_accesskit(1_000.0);
    harness.run_steps(3);
    let fitted = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
    assert!(fitted.width() < initial.width() * 0.8);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Auto fit");
    click(&mut harness, "Load");
    harness
        .get_by_role_and_label(Role::Slider, "Nodes")
        .set_value_accesskit(100.0);
    harness.run_steps(3);
    let fixed = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
    assert!(fixed.min.distance(fitted.min) < 0.1);
    assert!(fixed.max.distance(fitted.max) < 0.1);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Auto fit");
    let restored = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
    assert!(restored.min.distance(initial.min) < 0.1);
    assert!(restored.max.distance(initial.max) < 0.1);
}

#[test]
fn load_and_rendering_controls_apply_and_reset_statistics() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    harness.run_steps(20);
    click(&mut harness, "Load");
    harness
        .get_by_role_and_label(Role::Slider, "Nodes")
        .set_value_accesskit(500.0);
    harness.run_steps(3);
    assert_eq!(harness.state().scene().node_count(), 500);
    assert!(!harness.state().auto_grow());
    assert!(harness.state().sample_count() < 10);
    harness
        .get_by_role_and_label(Role::Slider, "Edges / node")
        .set_value_accesskit(5.0);
    harness.run_steps(3);
    assert_eq!(harness.state().scene().edge_count(), 2_500);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Nodes");
    click(&mut harness, "Diamonds");
    assert_eq!(harness.state().scene().appearance().kind, NodeKind::Diamond);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Edges");
    click(&mut harness, "Orthogonal · 90°");
    assert_eq!(harness.state().scene().routing(), RoutingMode::Orthogonal);
    click(&mut harness, "Forward arrows");
    assert_eq!(
        harness.state().scene().direction(),
        pautinka_egui::Direction::Forward
    );
}

#[test]
fn corner_radius_updates_geometry_and_survives_routing_changes_and_restart() {
    let scenario: Scenario = "[motion]\nanimate = false\n[growth]\nenabled = false"
        .parse()
        .unwrap();
    let mut harness = scenario_harness(scenario);
    click(&mut harness, "Edges");
    assert!(
        harness
            .get_by_role_and_label(Role::Slider, "Corner radius")
            .accesskit_node()
            .is_disabled()
    );
    for routing in ["Orthogonal · 90°", "Octilinear · 45°", "Angular · 15°"] {
        click(&mut harness, routing);
        let sharp = harness.state().scene().edge(0).path.points().to_vec();
        harness
            .get_by_role_and_label(Role::Slider, "Corner radius")
            .set_value_accesskit(12.0);
        harness.run_steps(3);
        assert_eq!(harness.state().scene().style().corner_radius.get(), 12.0);
        assert_ne!(harness.state().scene().edge(0).path.points(), sharp);
        harness
            .get_by_role_and_label(Role::Slider, "Corner radius")
            .set_value_accesskit(0.0);
        harness.run_steps(3);
        assert_eq!(harness.state().scene().edge(0).path.points(), sharp);
    }
    harness
        .get_by_role_and_label(Role::Slider, "Corner radius")
        .set_value_accesskit(20.0);
    harness.run_steps(3);
    for routing in ["Straight", "Curved"] {
        click(&mut harness, routing);
        assert!(
            harness
                .get_by_role_and_label(Role::Slider, "Corner radius")
                .accesskit_node()
                .is_disabled()
        );
        assert_eq!(harness.state().scene().style().corner_radius.get(), 20.0);
    }
    click(&mut harness, "Orthogonal · 90°");
    let rounded = harness.state().scene().edge(0).path.points().to_vec();
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Restart");
    assert_eq!(harness.state().scene().style().corner_radius.get(), 20.0);
    assert_eq!(harness.state().scene().edge(0).path.points(), rounded);
}

#[test]
fn curved_crossings_can_be_changed_and_survive_routing_changes_and_restart() {
    for (initial, label, other) in [("gap", "Gaps", "Bridges"), ("bridge", "Bridges", "Gaps")] {
        let scenario: Scenario = format!(
            "[motion]\nanimate = false\n[growth]\nenabled = false\n[edges]\nrouting = 'curved'\ncrossings = '{initial}'"
        ).parse().unwrap();
        let mut harness = scenario_harness(scenario);
        click(&mut harness, "Edges");
        assert_eq!(harness.state().scene().routing(), RoutingMode::Curved);
        assert_eq!(
            harness.get_by_label(label).accesskit_node().toggled(),
            Some(Toggled::True)
        );
        for selected in [other, "No decoration", label] {
            harness.get_by_label(selected).scroll_to_me();
            harness.run_steps(3);
            assert!(
                !harness
                    .get_by_label(selected)
                    .accesskit_node()
                    .is_disabled()
            );
            click(&mut harness, selected);
            assert_eq!(
                harness.get_by_label(selected).accesskit_node().toggled(),
                Some(Toggled::True)
            );
        }
        for routing in ["Straight", "Orthogonal · 90°", "Curved"] {
            harness.get_by_label(routing).scroll_to_me();
            harness.run_steps(3);
            click(&mut harness, routing);
            assert_eq!(
                harness.get_by_label(label).accesskit_node().toggled(),
                Some(Toggled::True)
            );
        }
        harness.key_press(egui::Key::Escape);
        harness.run_steps(3);
        click(&mut harness, "Restart");
        click(&mut harness, "Edges");
        assert_eq!(harness.state().scene().routing(), RoutingMode::Curved);
        assert_eq!(
            harness.get_by_label(label).accesskit_node().toggled(),
            Some(Toggled::True)
        );
    }
}

#[test]
fn hud_text_refreshes_with_statistics_and_resets_after_a_setting_change() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    harness.get_by_label("SAMPLES 0/600");
    harness.run_steps(3);
    harness.get_by_label("SAMPLES 0/600");
    harness.run_steps(20);
    let count = harness.state().sample_count();
    assert!(count > 0);
    harness.get_by_label(&format!("SAMPLES {count}/600"));
    click(&mut harness, "Nodes");
    click(&mut harness, "Diamonds");
    assert_eq!(harness.state().sample_count(), 0);
    harness.get_by_label("SAMPLES 0/600");
}

#[test]
fn real_drag_moves_an_orbit_and_escape_cancels_the_next_gesture() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    click(&mut harness, "Auto fit");
    click(&mut harness, "Animate");
    click(&mut harness, "Auto grow");
    let before = harness.state().scene().node(0).position;
    let from = harness
        .get_by_role_and_label(Role::Button, "Node 0")
        .rect()
        .center();
    let to = from + Vec2::new(37.0, 19.0);
    harness.event(Event::PointerMoved(from));
    harness.event(pointer(from, true));
    harness.step();
    harness.event(Event::PointerMoved(to));
    harness.run_steps(2);
    assert!(harness.state().scene().dragging());
    harness.event(pointer(to, false));
    harness.run_steps(3);
    let moved = harness.state().scene().node(0).position;
    assert!(moved.distance(before) > 1.0);
    let from = harness
        .get_by_role_and_label(Role::Button, "Node 0")
        .rect()
        .center();
    harness.event(Event::PointerMoved(from));
    harness.event(pointer(from, true));
    harness.step();
    harness.event(Event::PointerMoved(from + Vec2::new(-43.0, 31.0)));
    harness.run_steps(2);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(2);
    harness.event(pointer(from, false));
    harness.run_steps(3);
    assert_eq!(harness.state().scene().node(0).position, moved);
    click(&mut harness, "Animate");
    harness.run_steps(3);
    assert!(harness.state().scene().node(0).position.distance(moved) < 4.0);
    assert_ne!(harness.state().scene().node(0).position, moved);
}

#[test]
fn narrow_viewport_keeps_the_canvas_and_settings_accessible() {
    let mut harness = harness(Vec2::new(360.0, 640.0));
    harness.run_steps(3);
    let canvas = harness.get_by_label("Animated graph").rect();
    assert!(canvas.height() > 500.0);
    assert!(canvas.width() <= 360.0);
    let hud = harness.get_by_label("Performance HUD").rect();
    assert!(canvas.contains_rect(hud));
    let row_y = harness.get_by_label("Animate").rect().center().y;
    harness.get_by_label("View").scroll_to_me();
    harness.run_steps(20);
    let view = harness.get_by_label("View").rect();
    assert!((view.center().y - row_y).abs() < 1.0);
    assert!(view.left() >= 0.0 && view.right() <= 360.0);
    click(&mut harness, "View");
    click(&mut harness, "Labels");
    click(&mut harness, "Minimap");
    assert!(harness.query_by_label("Navigator").is_none());
    click(&mut harness, "Minimap");
    assert!(harness.query_by_label("Navigator").is_some());
    assert_eq!(harness.state().scene().load(), Load::default());
}

#[test]
fn lod_minimap_and_grid_are_enabled_by_default() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    assert!(harness.query_by_label("pautinka / stress").is_none());
    let row_y = harness.get_by_label("Animate").rect().center().y;
    for label in [
        "Auto grow",
        "Restart",
        "Auto fit",
        "Load",
        "Topology",
        "Motion",
        "Nodes",
        "Edges",
        "View",
    ] {
        assert!((harness.get_by_label(label).rect().center().y - row_y).abs() < 1.0);
    }
    assert!(harness.query_by_label("Navigator").is_some());
    click(&mut harness, "View");
    for label in ["Minimap", "Grid", "Overview simplification (LOD)"] {
        assert_eq!(
            harness.get_by_label(label).accesskit_node().toggled(),
            Some(Toggled::True),
            "{label} must be enabled by default",
        );
    }
}

#[test]
fn middle_click_fits_the_graph_after_zoom_including_through_the_hud() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    click(&mut harness, "Animate");
    click(&mut harness, "Auto grow");
    click(&mut harness, "Auto fit");
    let fitted = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
    let viewport = harness.get_by_label("Animated graph").rect();
    let hud = harness.get_by_label("Performance HUD").rect();
    for position in [viewport.center(), hud.center()] {
        harness.event(Event::PointerMoved(position));
        harness.step();
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, 80.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        });
        harness.run_steps(40);
        let zoomed = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
        assert!(zoomed.width() > fitted.width() + 1.0);
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: position,
                button: PointerButton::Middle,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.step();
        }
        harness.run_steps(3);
        let restored = harness.get_by_role_and_label(Role::Button, "Node 0").rect();
        assert!(
            restored.min.distance(fitted.min) < 0.1,
            "position={position:?}, fitted={fitted:?}, zoomed={zoomed:?}, restored={restored:?}"
        );
        assert!(restored.max.distance(fitted.max) < 0.1);
    }
}

#[test]
fn hud_sits_over_the_canvas_and_scroll_zoom_passes_through_it() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    click(&mut harness, "Auto fit");
    click(&mut harness, "Animate");
    click(&mut harness, "Auto grow");
    let viewport = harness.get_by_label("Animated graph").rect();
    let hud = harness.get_by_label("Performance HUD").rect();
    assert!(viewport.height() > 850.0);
    assert!(hud.min.distance(viewport.min + Vec2::splat(12.0)) < 1.0);
    assert!(viewport.contains_rect(hud));
    let before = harness
        .get_by_role_and_label(Role::Button, "Node 0")
        .rect()
        .center();
    harness.event(Event::PointerMoved(hud.center()));
    harness.step();
    harness.event(Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, 80.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(10);
    let after = harness
        .get_by_role_and_label(Role::Button, "Node 0")
        .rect()
        .center();
    assert!(after.distance(before) > 1.0);
}

#[test]
fn fractional_density_and_topology_controls_apply_and_survive_restart() {
    let mut harness = harness(Vec2::new(1440.0, 960.0));
    click(&mut harness, "Auto grow");
    click(&mut harness, "Load");
    harness
        .get_by_role_and_label(Role::Slider, "Edges / node")
        .set_value_accesskit(0.35);
    harness.run_steps(3);
    assert_eq!(harness.state().scene().edge_count(), 35);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Topology");
    click(&mut harness, "Random");
    for (label, value) in [
        ("Local links", 78.0),
        ("Local radius", 6.0),
        ("Hub links", 60.0),
        ("Hub nodes", 3.0),
    ] {
        harness
            .get_by_role_and_label(Role::Slider, label)
            .set_value_accesskit(value);
        harness.run_steps(3);
    }
    let topology = pautinka_stress::scene::Topology::new(78, 6, 60, 3, false).unwrap();
    assert_eq!(harness.state().scene().topology(), topology);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Restart");
    assert_eq!(harness.state().scene().topology(), topology);
    assert_eq!(harness.state().scene().edge_count(), 35);
}
