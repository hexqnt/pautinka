#[cfg(all(test, feature = "count-allocations"))]
mod tests {
    use super::*;
    use pautinka_test_support::canvas::Scene;
    use std::{hint::black_box, num::NonZeroUsize};

    #[test]
    fn alignment_queries_and_buffer_reuse_do_not_allocate() {
        use pautinka_egui::{SnapGrid, SnapIndex, SnapKind, SnapKinds, SnapOptions};

        let scene = Scene::new(SceneKind::Network, NonZeroUsize::new(10_000).unwrap());
        let mut index = SnapIndex::default();
        index.rebuild(scene.positions.iter().copied(), SnapKinds::ALL);
        let modes: [SnapKinds; 16] = std::array::from_fn(|mask| {
            SnapKind::ALL
                .into_iter()
                .enumerate()
                .fold(SnapKinds::NONE, |mut kinds, (bit, kind)| {
                    kinds.set(kind.flag(), mask & (1 << bit) != 0);
                    kinds
                })
        });
        let grid = SnapGrid::new(10.0).unwrap();
        let info = allocation_counter::measure(|| {
            let mut previous = None;
            for prepared in modes {
                index.rebuild(scene.positions.iter().copied(), prepared);
                for kinds in modes {
                    let policy = SnapOptions {
                        kinds,
                        ..SnapOptions::default()
                    };
                    for &point in scene.positions.iter().step_by(37) {
                        let result = index.snap(black_box(point), policy, previous);
                        previous = Some(black_box(result));
                        let result =
                            index.snap_with_grid(black_box(point), policy, Some(grid), previous);
                        previous = Some(black_box(result));
                    }
                }
            }
        });
        assert_eq!(info.count_total, 0);
        assert_eq!(info.bytes_total, 0);
    }

    #[test]
    fn indexed_picking_does_not_allocate_after_preparation() {
        let scene = Scene::new(SceneKind::Network, NonZeroUsize::new(10_000).unwrap());
        let mut canvas = pautinka_egui::Canvas::default();
        canvas.prepare(&scene).unwrap();
        let info = allocation_counter::measure(|| {
            for &point in scene.positions.iter().step_by(37) {
                black_box(canvas.pick(black_box(point), black_box(&scene)));
            }
        });
        assert_eq!(info.count_total, 0);
        assert_eq!(info.bytes_total, 0);
    }

    #[test]
    fn bezier_hit_refinement_uses_no_heap_memory() {
        let controls = [
            egui::pos2(0.0, 0.0),
            egui::pos2(500.0, 1000.0),
            egui::pos2(1000.0, 0.0),
        ];
        let path = pautinka_egui::EdgePath::try_quadratic(controls).unwrap();
        let info = allocation_counter::measure(|| {
            for step in 0..97 {
                let t = step as f32 / 96.0;
                let [a, b, c] = controls;
                let point = a.lerp(b, t).lerp(b.lerp(c, t), t);
                for chunk in 0..path.spatial_chunks().len() {
                    black_box(path.chunk_hit_distance_sq(chunk, black_box(point), 0.35));
                }
            }
        });
        assert_eq!(info.count_total, 0);
    }

    #[test]
    fn compact_routes_keep_straight_and_coincident_paths_inline() {
        use pautinka_egui::{RouteEndpoints, RoutingMode};

        let info = allocation_counter::measure(|| {
            for mode in [
                RoutingMode::Curved,
                RoutingMode::Straight,
                RoutingMode::Orthogonal,
                RoutingMode::Octilinear,
                RoutingMode::Angular15,
            ] {
                for to in [
                    egui::Pos2::ZERO,
                    egui::pos2(100.0, 0.0),
                    egui::pos2(0.0, 100.0),
                ] {
                    black_box(
                        mode.route(
                            RouteEndpoints::Between {
                                from: egui::Pos2::ZERO,
                                to,
                            },
                            0.0,
                            0,
                        )
                        .unwrap(),
                    );
                }
            }
        });
        assert_eq!(info.count_total, 0);
        assert_eq!(info.bytes_total, 0);
    }

    #[test]
    fn stress_animation_and_drag_reuse_model_buffers() {
        use pautinka_egui::{CanvasEvent, Graph, NodeId, RoutingMode};
        use pautinka_stress::scene::{Load, Motion, Scene};
        use std::time::Duration;

        let mut scene = Scene::new(42, Load::new(5_000, 3.75).unwrap());
        let step = Duration::from_secs_f64(1.0 / 60.0);
        let info = allocation_counter::measure(|| {
            for routing in [RoutingMode::Straight, RoutingMode::Curved] {
                scene.set_routing(routing);
                for fraction in [1, 50, 100] {
                    let motion = Motion::new(fraction, 1.0).unwrap();
                    for _ in 0..10 {
                        scene.advance(step, motion);
                        black_box(scene.bounds());
                    }
                }
                let node = NodeId::new(0);
                let position = scene.node(0).position;
                scene.handle_event(CanvasEvent::NodeDragStarted { node, position });
                scene.handle_event(CanvasEvent::NodeDragged {
                    node,
                    position: position + egui::vec2(10.0, -20.0),
                });
                scene.handle_event(CanvasEvent::NodeDragCancelled { node });
            }
        });
        assert_eq!(info.count_total, 0);
        assert_eq!(info.bytes_total, 0);
    }

    #[test]
    fn measurement_returns_the_value_and_records_allocations_after_the_scope() {
        let mut csv = String::new();
        let value = measured(
            &mut csv,
            Scope {
                scene: SceneKind::Network,
                nodes: 1,
                view: "test",
                action: "allocate",
                frame: 0,
            },
            || black_box(vec![1_u8; 100]),
        );
        assert_eq!(value.len(), 100);
        let columns: Vec<_> = csv.trim().split(',').collect();
        assert!(columns[6].parse::<u64>().unwrap() >= 1);
        assert!(columns[7].parse::<u64>().unwrap() >= 100);
    }
}

use pautinka_test_support::canvas::SceneKind;

#[derive(Clone, Copy)]
pub struct Scope<'a> {
    pub scene: SceneKind,
    pub nodes: usize,
    pub view: &'a str,
    pub action: &'a str,
    pub frame: usize,
}

pub fn measured<T>(csv: &mut String, scope: Scope<'_>, run: impl FnOnce() -> T) -> T {
    #[cfg(feature = "count-allocations")]
    {
        use std::fmt::Write as _;
        let mut value = None;
        let info = allocation_counter::measure(|| value = Some(run()));
        writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{}",
            scope.scene,
            scope.nodes,
            scope.nodes * 3,
            scope.view,
            scope.action,
            scope.frame,
            info.count_total,
            info.bytes_total,
            info.bytes_current,
            info.bytes_max
        )
        .unwrap();
        value.expect("Allocation measurement must execute its closure")
    }
    #[cfg(not(feature = "count-allocations"))]
    {
        let _ = (
            csv,
            scope.scene,
            scope.nodes,
            scope.view,
            scope.action,
            scope.frame,
        );
        run()
    }
}
