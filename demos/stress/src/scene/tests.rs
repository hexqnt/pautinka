use pautinka::egui::{ChangeCoverage, ElementId, Graph};

use super::*;

#[test]
fn generated_network_is_deterministic_connected_and_has_no_duplicate_targets() {
    for density in [1_u8, 3, 8] {
        let load = Load::new(250, density).unwrap();
        let first = Scene::new(42, load);
        let second = Scene::new(42, load);
        assert_eq!(first.edge_count(), 250 * usize::from(density));
        assert_eq!(first.bounds(), second.bounds());
        for (a, b) in first.nodes.iter().zip(&second.nodes) {
            assert_eq!(a.position, b.position);
        }
        for edges in first.edges.chunks_exact(usize::from(density)) {
            let mut targets = Vec::new();
            for edge in edges {
                let [a, b] = edge.ends;
                assert_ne!(a, b);
                assert!(!targets.contains(&b));
                assert!(b < first.node_count());
                targets.push(b);
            }
            let source = edges[0].ends[0];
            if source > 0 {
                assert_eq!(edges[0].ends[1], source - 1);
            }
        }
        for (a, b) in first.edges.iter().zip(&second.edges) {
            assert_eq!(a.ends, b.ends);
            assert_eq!(a.path.points(), b.path.points());
        }
    }
    assert_ne!(
        Scene::new(42, Load::default()).node(0).position,
        Scene::new(43, Load::default()).node(0).position
    );
}

#[test]
fn growth_keeps_four_to_three_proportions_and_existing_orbit_centers() {
    let mut scene = Scene::new(42, Load::new(STAGES[0], 1).unwrap());
    let initial_centers: Vec<_> = scene.nodes.iter().map(|node| node.center).collect();
    let target_ratio = 4.0 / 3.0;
    for count in STAGES {
        scene.resize(Load::new(count, 1).unwrap());
        let bounds = scene.nodes.iter().fold(Rect::NOTHING, |bounds, node| {
            bounds.union(Rect::from_min_max(node.center, node.center))
        });
        // Незавершённый внешний виток даёт дискретное отклонение пропорций.
        assert!((bounds.width() / bounds.height() / target_ratio - 1.0).abs() < 0.08);
        for (node, &center) in scene.nodes.iter().zip(&initial_centers) {
            assert_eq!(node.center, center);
        }
    }
}

#[test]
fn animation_updates_each_incident_edge_once_and_keeps_full_change_coverage() {
    let mut scene = Scene::new(42, Load::new(1_000, 3).unwrap());
    let before = scene.revision();
    scene.advance(Duration::from_secs_f64(1.0 / 60.0), Motion::default());
    let mut changed = Vec::new();
    assert_eq!(
        scene.geometry_changes(before, &mut |item| changed.push(item)),
        ChangeCoverage::Complete
    );
    assert_eq!(changed.len(), scene.node_count() + scene.edge_count());
    let next = scene.revision();
    scene.advance(Duration::from_secs_f64(1.0 / 60.0), Motion::default());
    assert_eq!(
        scene.geometry_changes(before, &mut |_| {}),
        ChangeCoverage::Complete
    );
    assert_eq!(
        scene.geometry_changes(next, &mut |_| {}),
        ChangeCoverage::Complete
    );
    let revision = scene.revision();
    scene.advance(Duration::from_secs(1), Motion::new(0, 1.0).unwrap());
    scene.advance(Duration::from_secs(1), Motion::new(100, 0.0).unwrap());
    assert_eq!(scene.revision(), revision);
}

#[test]
fn animation_preserves_wrapped_orbits_and_exact_bounds() {
    let mut scene = Scene::new(42, Load::new(250, 3.75).unwrap());
    for routing in [
        RoutingMode::Curved,
        RoutingMode::Straight,
        RoutingMode::Orthogonal,
    ] {
        scene.set_routing(routing);
        for fraction in [1, 50, 100] {
            let motion = Motion::new(fraction, 3.0).unwrap();
            for step in [Duration::from_millis(16), Duration::from_secs(1_000_000)] {
                let expected: Vec<_> = scene
                    .nodes
                    .iter()
                    .map(|node| {
                        if node.moving_rank >= fraction {
                            return (node.phase, node.position);
                        }
                        let phase = (node.phase
                            + step.as_secs_f64() * f64::from(motion.speed) * node.frequency)
                            % TAU;
                        let (sin, cos) = phase.sin_cos();
                        let position = node.center
                            + Vec2::new(
                                cos as f32 * node.amplitude.x,
                                sin as f32 * node.amplitude.y,
                            );
                        (phase, position)
                    })
                    .collect();
                scene.advance(step, motion);
                for (node, (phase, position)) in scene.nodes.iter().zip(expected) {
                    assert_eq!(node.phase, phase);
                    assert_eq!(node.position, position);
                }
                let bounds = scene.bounds();
                scene.update_bounds();
                assert_eq!(scene.bounds(), bounds);
            }
        }
    }
}

#[test]
fn dragging_freezes_phase_resumes_without_a_jump_and_escape_restores_the_orbit() {
    let mut scene = Scene::new(42, Load::default());
    let node = NodeId::new(3);
    let position = scene.node(3).position;
    let center = scene.nodes[3].center;
    let phase = scene.nodes[3].phase;
    let target = position + Vec2::new(123.0, -74.0);
    scene.handle_event(CanvasEvent::NodeDragStarted { node, position });
    scene.handle_event(CanvasEvent::NodeDragged {
        node,
        position: target,
    });
    scene.advance(Duration::from_millis(500), Motion::default());
    assert_eq!(scene.node(3).position, target);
    assert_eq!(scene.nodes[3].phase, phase);
    scene.handle_event(CanvasEvent::NodeDragCancelled { node });
    assert_eq!(scene.node(3).position, position);
    assert_eq!(scene.nodes[3].center, center);
    scene.handle_event(CanvasEvent::NodeDragStarted { node, position });
    scene.handle_event(CanvasEvent::NodeDragFinished {
        node,
        position: target,
    });
    assert_eq!(scene.node(3).position, target);
    scene.advance(Duration::from_secs_f64(1.0 / 60.0), Motion::default());
    assert!(scene.node(3).position.distance(target) < 1.0);
    assert!(scene.nodes[3].center.distance(center + (target - position)) < 0.0001);
    let moved = scene.node(3).position;
    scene.resize(Load::new(500, 8).unwrap());
    assert_eq!(scene.node(3).position, moved);
    assert_eq!(
        scene.resolve(ElementId::Node(node)),
        Some(Selection::Node(3))
    );
}

#[test]
fn all_routes_and_appearance_changes_preserve_finite_geometry_and_identity() {
    let mut scene = Scene::new(u32::MAX, Load::new(100, 8).unwrap());
    let id = scene.edge(10).id;
    for routing in [
        RoutingMode::Straight,
        RoutingMode::Curved,
        RoutingMode::Orthogonal,
        RoutingMode::Octilinear,
        RoutingMode::Angular15,
    ] {
        scene.set_routing(routing);
        scene.advance(Duration::from_secs_f64(1.0 / 60.0), Motion::default());
        assert!(scene.bounds().is_finite());
        for edge in &scene.edges {
            assert!(edge.path.bounds().is_finite());
            assert_eq!(
                edge.path.points().first(),
                Some(&scene.nodes[edge.ends[0]].position)
            );
            assert_eq!(
                edge.path.points().last(),
                Some(&scene.nodes[edge.ends[1]].position)
            );
        }
    }
    scene.resize(Load::new(250, 3).unwrap());
    assert_eq!(scene.resolve(ElementId::Edge(id)), Some(Selection::Edge(5)));
    let revision = scene.revision();
    scene.set_appearance(Appearance::new(NodeKind::Diamond, 12, 16.0).unwrap());
    assert_eq!(
        scene.geometry_changes(revision, &mut |_| {}),
        ChangeCoverage::Complete
    );
    assert!(Load::new(99, 3).is_err());
    assert!(Load::new(100, 16.01).is_err());
    assert!(Motion::new(101, 1.0).is_err());
    assert!(Motion::new(10, f32::NAN).is_err());
    assert!(Appearance::new(NodeKind::Circle, 0, 7.0).is_err());
}

#[test]
fn rounding_changes_publish_edges_and_persist_through_motion_and_growth() {
    for routing in [
        RoutingMode::Orthogonal,
        RoutingMode::Octilinear,
        RoutingMode::Angular15,
    ] {
        let mut scene = Scene::with_style(
            42,
            Load::default(),
            SceneStyle {
                routing,
                ..SceneStyle::default()
            },
        );
        let sharp = scene.edge(0).path.points().to_vec();
        let revision = scene.revision();
        let style = SceneStyle {
            corner_radius: pautinka::egui::CornerRadius::try_from(12.0).unwrap(),
            ..scene.style()
        };
        scene.set_style(style);
        assert_ne!(scene.edge(0).path.points(), sharp);
        let mut changed = Vec::new();
        assert_eq!(
            scene.geometry_changes(revision, &mut |item| changed.push(item)),
            ChangeCoverage::Complete
        );
        assert_eq!(
            changed,
            (0..scene.edge_count())
                .map(Selection::Edge)
                .collect::<Vec<_>>()
        );
        scene.set_style(SceneStyle {
            corner_radius: pautinka::egui::CornerRadius::default(),
            ..style
        });
        assert_eq!(scene.edge(0).path.points(), sharp);
        scene.set_style(style);
        scene.advance(Duration::from_secs_f64(1.0 / 60.0), Motion::default());
        for load in [Load::default(), Load::new(250, 3).unwrap()] {
            scene.resize(load);
            for edge in &scene.edges {
                let sharp_style = SceneStyle {
                    corner_radius: pautinka::egui::CornerRadius::default(),
                    ..style
                };
                let expected = route(sharp_style, &scene.nodes, edge.ends, edge.lane)
                    .with_corner_radius(style.corner_radius)
                    .unwrap();
                assert_eq!(edge.path.points(), expected.points());
                assert!(scene.bounds().contains_rect(edge.path.bounds()));
            }
        }
    }
}

#[test]
fn styled_construction_and_batched_changes_produce_the_same_graph() {
    let load = Load::new(250, 5).unwrap();
    let style = SceneStyle {
        appearance: Appearance::new(NodeKind::Hexagon, 8, 12.0).unwrap(),
        routing: RoutingMode::Orthogonal,
        corner_radius: pautinka::egui::CornerRadius::try_from(10.0).unwrap(),
        direction: Direction::Both,
    };
    let direct = Scene::with_style(42, load, style);
    let mut updated = Scene::new(42, load);
    let before = updated.revision();
    updated.set_style(style);
    let mut changed = Vec::new();
    assert_eq!(
        updated.geometry_changes(before, &mut |item| changed.push(item)),
        ChangeCoverage::Complete
    );
    assert_eq!(changed.len(), load.nodes() + load.edge_count());
    assert_eq!(updated.revision().geometry(), before.geometry() + 1);
    for index in 0..load.nodes() {
        assert_eq!(direct.node(index).position, updated.node(index).position);
        assert_eq!(direct.node_shape(index), updated.node_shape(index));
    }
    for index in 0..load.edge_count() {
        assert_eq!(
            direct.edge(index).path.points(),
            updated.edge(index).path.points()
        );
    }
    let revision = updated.revision();
    updated.resize(load);
    updated.set_style(style);
    assert_eq!(updated.revision(), revision);
}

#[test]
fn fractional_density_has_exact_counts_unique_targets_and_resolvable_ids() {
    for density in [0.0_f64, 0.01, 0.35, 0.99, 1.25, 3.75, 16.0] {
        let load = Load::new(137, density).unwrap();
        let scene = Scene::new(42, load);
        assert_eq!(
            scene.edge_count(),
            usize::try_from((137.0 * density).round() as i32).unwrap()
        );
        let mut cursor = 0;
        for source in 0..scene.node_count() {
            let range = load.edge_range(source);
            assert_eq!(range.start, cursor);
            let outgoing = &scene.edges[range.clone()];
            for (lane, edge) in outgoing.iter().enumerate() {
                assert_eq!(edge.ends[0], source);
                assert_ne!(edge.ends[1], source);
                assert!(
                    !outgoing[..lane]
                        .iter()
                        .any(|earlier| earlier.ends[1] == edge.ends[1])
                );
                assert_eq!(
                    scene.resolve(ElementId::Edge(scene.edge(range.start + lane).id)),
                    Some(Selection::Edge(range.start + lane))
                );
            }
            cursor = range.end;
        }
        assert_eq!(cursor, scene.edge_count());
        assert_eq!(
            scene.resolve(ElementId::Edge(pautinka::egui::EdgeId::new(
                137 * u64::from(Load::MAX_EDGES_PER_NODE)
            ))),
            None
        );
    }
    assert!(Load::new(100, f64::NAN).is_err());
    assert!(Load::new(100, -0.1).is_err());
}

#[test]
fn topology_changes_preserve_particles_and_survive_growth() {
    let mut scene = Scene::new(42, Load::default());
    let before: Vec<_> = scene.nodes.iter().map(|node| node.position).collect();
    let topology = Topology::new(25, 6, 60, 3, false).unwrap();
    let revision = scene.revision();
    scene.reconfigure(scene.load(), topology);
    assert!(scene.revision().topology() > revision.topology());
    for (node, position) in scene.nodes.iter().zip(before) {
        assert_eq!(node.position, position);
    }
    let replica = Scene::with_options(42, scene.load(), scene.style(), topology);
    for (edge, other) in scene.edges.iter().zip(replica.edges) {
        assert_eq!(edge.ends, other.ends);
    }
    scene.resize(scene.load().next_stage().unwrap());
    assert_eq!(scene.topology(), topology);
    assert!(Topology::new(101, 1, 0, 1, false).is_err());
    assert!(Topology::new(100, 0, 0, 1, false).is_err());
    assert!(Topology::new(100, 1, 0, 0, false).is_err());
}
