use super::*;

fn close_requested(harness: &Harness<'_, Demo>) -> bool {
    harness.output().viewport_output.values().any(|output| {
        output
            .commands
            .iter()
            .any(|command| matches!(command, egui::ViewportCommand::Close))
    })
}

#[test]
fn configured_growth_holds_the_exact_target_then_closes_the_window() {
    let scenario: Scenario = "[run]\nmax_nodes = 350\nhold_secs = 2\n[growth]\ninterval_secs = 10"
        .parse()
        .unwrap();
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1440.0, 960.0))
        .with_step_dt(1.0)
        .build_ui_state(|ui, demo| demo.show(ui), Demo::with_scenario(scenario));
    let mut reached = None;
    let mut saw_intermediate_stage = false;
    for frame in 0..25 {
        let nodes = harness.state().scene().node_count();
        assert!(nodes <= 350);
        saw_intermediate_stage |= nodes == 250;
        if nodes == 350 {
            reached.get_or_insert(frame);
        }
        if close_requested(&harness) {
            assert!(saw_intermediate_stage);
            assert!(frame - reached.unwrap() >= 2);
            return;
        }
        harness.step();
    }
    panic!("Scenario did not close the window after holding the target");
}

#[test]
fn configured_duration_closes_before_a_later_growth_target() {
    let scenario: Scenario =
        "[run]\nduration_secs = 10\nmax_nodes = 5000\nhold_secs = 10\n[growth]\ninterval_secs = 30"
            .parse()
            .unwrap();
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1440.0, 960.0))
        .with_step_dt(1.0)
        .build_ui_state(|ui, demo| demo.show(ui), Demo::with_scenario(scenario));
    assert!(!close_requested(&harness));
    for _ in 0..12 {
        harness.step();
        assert_eq!(harness.state().scene().node_count(), 100);
        if close_requested(&harness) {
            return;
        }
    }
    panic!("Scenario did not close the window at its duration limit");
}

#[test]
fn initial_target_with_zero_hold_closes_without_growth() {
    let scenario: Scenario = "[run]\nmax_nodes = 100\nhold_secs = 0\n[growth]\nenabled = false"
        .parse()
        .unwrap();
    let harness = scenario_harness(scenario);
    assert!(close_requested(&harness));
    assert_eq!(harness.state().scene().node_count(), 100);
    assert!(!harness.state().auto_grow());
}

#[test]
fn configured_step_duration_remains_available_after_adjusting_the_slider() {
    for seconds in [0.5, 60.0] {
        let scenario: Scenario = format!("[growth]\nenabled = false\ninterval_secs = {seconds}")
            .parse()
            .unwrap();
        let mut harness = scenario_harness(scenario);
        click(&mut harness, "Load");
        for value in [10.0, seconds] {
            harness
                .get_by_role_and_label(Role::Slider, "Step duration")
                .set_value_accesskit(value);
            harness.run_steps(3);
            assert_eq!(
                harness
                    .get_by_role_and_label(Role::Slider, "Step duration")
                    .accesskit_node()
                    .numeric_value(),
                Some(value)
            );
        }
    }
}

#[test]
fn scenario_settings_reach_the_scene_controls_and_restart() {
    let scenario: Scenario = r#"
seed = 7
[load]
nodes = 250
edges_per_node = 2.5
[growth]
enabled = false
interval_secs = 0.5
[topology]
locality = 20
hub_share = 70
hubs = 3
backbone = false
[motion]
animate = false
fraction = 50
speed = 0.5
[nodes]
kind = "diamond"
colors = 4
radius = 9.0
[edges]
routing = "orthogonal"
direction = "both"
pattern = "dashed"
corner_radius = 12.0
crossings = "gap"
[view]
auto_fit = false
labels = true
lod = false
minimap = false
grid = false
"#
    .parse()
    .unwrap();
    let mut harness = scenario_harness(scenario);
    let scene = harness.state().scene();
    assert_eq!(scene.node_count(), 250);
    assert_eq!(scene.edge_count(), 625);
    assert_eq!(scene.appearance().kind, NodeKind::Diamond);
    assert_eq!(scene.appearance().colors(), 4);
    assert_eq!(scene.appearance().radius(), 9.0);
    assert_eq!(scene.routing(), RoutingMode::Orthogonal);
    assert_eq!(scene.style().corner_radius.get(), 12.0);
    assert_eq!(scene.direction(), pautinka_egui::Direction::Both);
    assert_eq!(
        scene.topology(),
        pautinka_stress::scene::Topology::new(20, 3, 70, 3, false).unwrap()
    );
    assert!(!harness.state().running());
    assert!(!harness.state().auto_grow());
    assert!(harness.query_by_label("Navigator").is_none());
    click(&mut harness, "Load");
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Slider, "Step duration")
            .accesskit_node()
            .numeric_value(),
        Some(0.5),
    );
    assert_eq!(harness.state().scene().node_count(), 250);
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "View");
    for (label, expected) in [
        ("Labels", Toggled::True),
        ("Minimap", Toggled::False),
        ("Grid", Toggled::False),
        ("Overview simplification (LOD)", Toggled::False),
    ] {
        assert_eq!(
            harness.get_by_label(label).accesskit_node().toggled(),
            Some(expected)
        );
    }
    harness.key_press(egui::Key::Escape);
    harness.run_steps(3);
    click(&mut harness, "Restart");
    assert_eq!(harness.state().scene().node_count(), 250);
    assert_eq!(harness.state().scene().edge_count(), 625);
    assert!(!close_requested(&harness));
}
