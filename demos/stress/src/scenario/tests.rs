use super::*;

#[test]
fn empty_scenario_preserves_interactive_defaults() {
    let scenario: Scenario = "".parse().unwrap();
    assert_eq!(scenario.seed, 42);
    assert_eq!(Scenario::default().seed, scenario.seed);
    assert_eq!(scenario.load, Load::default());
    assert_eq!(scenario.topology, Topology::default());
    assert_eq!(scenario.style, SceneStyle::default());
    assert_eq!(scenario.style.corner_radius.get(), 0.0);
    assert_eq!(scenario.motion, Motion::default());
    assert_eq!(scenario.growth.interval, Duration::from_secs(10));
    assert!(scenario.growth.enabled);
    assert!(scenario.limits.duration.is_none());
    assert!(scenario.limits.target.is_none());
}

#[test]
fn checked_in_scenarios_parse_and_keep_their_limits() {
    let growth: Scenario = include_str!("../../scenarios/growth.toml").parse().unwrap();
    assert_eq!(growth.growth.maximum.get(), 5_000);
    assert_eq!(growth.growth.interval, Duration::from_secs(5));
    assert_eq!(growth.limits.target.unwrap().hold, Duration::from_secs(15));
    let fixed: Scenario = include_str!("../../scenarios/fixed.toml").parse().unwrap();
    assert!(!fixed.growth.enabled);
    assert_eq!(fixed.load.nodes(), 5_000);
    assert_eq!(fixed.limits.duration, Some(Duration::from_secs(30)));
    assert!(fixed.limits.target.is_none());
}

#[test]
fn curved_edges_accept_gap_and_bridge_scenarios() {
    for (value, expected) in [("gap", CrossingMode::Gap), ("bridge", CrossingMode::Bridge)] {
        let scenario: Scenario = format!("[edges]\nrouting = 'curved'\ncrossings = '{value}'")
            .parse()
            .unwrap();
        assert_eq!(scenario.style.routing, pautinka::egui::RoutingMode::Curved);
        assert_eq!(scenario.crossings, expected);
    }
}

#[test]
fn invalid_or_inconsistent_settings_fail_at_the_boundary() {
    for source in [
        "unknown = true",
        "[load]\nnode = 100",
        "[load]\nnodes = 99",
        "[load]\nnodes = 20001",
        "[load]\nedges_per_node = nan",
        "[topology]\nlocality = 101",
        "[topology]\nhubs = 0",
        "[motion]\nfraction = 101",
        "[motion]\nspeed = inf",
        "[nodes]\nkind = 'unknown'",
        "[nodes]\nradius = -1",
        "[edges]\nrouting = 'unknown'",
        "[edges]\npattern = 'unknown'",
        "[edges]\ncorner_radius = -1",
        "[edges]\ncorner_radius = nan",
        "[edges]\ncorner_radius = inf",
        "[edges]\ncrossings = 'unknown'",
        "[view]\nlabel = true",
        "[run]\nduration_secs = 0",
        "[run]\nduration_secs = inf",
        "[run]\nduration_secs = -1",
        "[run]\nduration_secs = 1e100",
        "[run]\nmax_nodes = 20001",
        "[run]\nhold_secs = 1",
        "[run]\nmax_nodes = 500\nhold_secs = -1",
        "[run]\nmax_nodes = 500\n[load]\nnodes = 1000",
        "[run]\nmax_nodes = 500\n[growth]\nenabled = false",
        "[growth]\ninterval_secs = 0",
        "[growth]\ninterval_secs = nan",
        "[growth]\ninterval_secs = 1e-100",
    ] {
        assert!(
            source.parse::<Scenario>().is_err(),
            "Accepted invalid scenario: {source}"
        );
    }
}

#[test]
fn fractional_seconds_and_an_initial_target_are_supported() {
    let scenario: Scenario = "[run]\nduration_secs = 0.5\nmax_nodes = 100\nhold_secs = 0\n[growth]\nenabled = false\ninterval_secs = 0.25".parse().unwrap();
    assert_eq!(scenario.limits.duration, Some(Duration::from_millis(500)));
    assert_eq!(scenario.limits.target.unwrap().hold, Duration::ZERO);
    assert_eq!(scenario.growth.interval, Duration::from_millis(250));
}

#[test]
fn invalid_limits_point_to_their_toml_values() {
    for (source, value) in [
        ("[run]\nduration_secs = 0", "0"),
        ("[run]\nmax_nodes = 99", "99"),
        ("[run]\nmax_nodes = 500\nhold_secs = -1", "-1"),
        ("[growth]\ninterval_secs = nan", "nan"),
        ("[edges]\ncorner_radius = -1", "-1"),
    ] {
        let error = source.parse::<Scenario>().unwrap_err();
        let span = error
            .span()
            .expect("Invalid value must have a source location");
        assert_eq!(&source[span], value, "{error}");
    }
}
