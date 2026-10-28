#[path = "edges/patterns.rs"]
mod patterns;

#[path = "edges/curves.rs"]
mod curves;

use super::*;

#[test]
fn crossing_gaps_render_at_constant_screen_size_and_remain_selectable() {
    let mut state = State::default();
    state.graph.second_is_loop = false;
    state.graph.nodes = vec![pos2(-150.0, -150.0), pos2(150.0, 150.0)];
    state.graph.edges = [
        EdgePath::try_line(pos2(0.0, -100.0), pos2(0.0, 100.0)).unwrap(),
        EdgePath::try_line(pos2(-100.0, 0.0), pos2(100.0, 0.0)).unwrap(),
    ];
    state.canvas.set_camera(pautinka_egui::geometry::Camera {
        center: Pos2::ZERO,
        zoom: 1.0,
        fit: false,
    });
    let mut harness = canvas_harness(state);
    for zoom in [1.0, 2.0] {
        harness.state_mut().canvas.camera.zoom = zoom;
        harness.run();
        let center = harness.get_by_label("Graph canvas").rect().center();
        let mut paths = harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| {
                let points = match &shape.shape {
                    egui::Shape::LineSegment { points, .. } => points.as_slice(),
                    egui::Shape::Path(path) => path.points.as_slice(),
                    _ => return None,
                };
                if points.len() == 2 && points.iter().all(|point| (point.y - center.y).abs() < 0.01)
                {
                    Some(points)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        paths.sort_by(|a, b| a[0].x.total_cmp(&b[0].x));
        assert_eq!(paths.len(), 2);
        assert!((paths[1][0].x - paths[0][1].x - 10.0).abs() < 0.01);
    }
    let viewport = harness.get_by_label("Graph canvas").rect();
    let pointer = viewport.center() + Vec2::new(2.0, 0.0);
    pointer_gesture(
        &mut harness,
        pointer,
        pointer,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(1)));
    harness.state_mut().canvas.style.crossings.mode = pautinka_egui::CrossingMode::None;
    harness.run();
    let center = harness.get_by_label("Graph canvas").rect().center();
    assert!(harness.output().shapes.iter().any(|shape| {
        let points = match &shape.shape {
            egui::Shape::LineSegment { points, .. } => points.as_slice(),
            egui::Shape::Path(path) => path.points.as_slice(),
            _ => return false,
        };
        points.len() == 2
            && points[0].x < center.x
            && points[1].x > center.x
            && points.iter().all(|point| (point.y - center.y).abs() < 0.01)
    }));
}

#[test]
fn arrow_culling_keeps_visible_edge_bodies_and_heads_with_pixel_feathering() {
    use egui::{Modifiers, PointerButton, Shape};
    use pautinka_egui::geometry::Camera;

    let edge_segments = |harness: &Harness<'_, State>| {
        harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| match shape.shape {
                Shape::LineSegment { points, stroke } if stroke.color == Color32::LIGHT_GREEN => {
                    Some(points)
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    for (pixels_per_point, feathering) in [(1.0, 1.0), (0.5, 8.0)] {
        let mut state = State::default();
        state.graph.nodes = vec![pos2(-2000.0, 0.0), pos2(2000.0, 0.0)];
        state.graph.edges[0] =
            EdgePath::try_line(state.graph.nodes[0], state.graph.nodes[1]).unwrap();
        state.canvas.set_show_loops(false, &state.graph);
        state.canvas.set_labels(false);
        state.canvas.set_camera(Camera {
            center: Pos2::ZERO,
            zoom: 1.0,
            fit: false,
        });
        let tip = state.graph.edges[0].arrow(false)[1];
        let mut harness = Harness::builder()
            .with_size(Vec2::new(800.0, 600.0))
            .with_pixels_per_point(pixels_per_point)
            .build_ui_state(
                move |ui, state: &mut State| {
                    ui.ctx().tessellation_options_mut(|options| {
                        options.feathering_size_in_pixels = feathering;
                    });
                    state.canvas.show(ui, &state.graph);
                },
                state,
            );
        harness.run();
        let segments = edge_segments(&harness);
        assert!(
            segments
                .iter()
                .any(|points| points[0].distance(points[1]) > 3000.0)
        );
        assert!(
            segments
                .iter()
                .all(|points| points[0].distance(points[1]) > 10.0)
        );

        if pixels_per_point < 1.0 {
            let viewport = harness.get_by_label("Graph canvas").rect();
            harness.state_mut().canvas.camera.center =
                tip - Vec2::new(viewport.width() * 0.5 + 15.0, 0.0);
            harness.run();
            let viewport = harness.get_by_label("Graph canvas").rect();
            let screen_tip = harness.state().canvas.camera.to_screen(tip, viewport);
            assert!(screen_tip.x > viewport.max.x + 12.0);
            assert!(edge_segments(&harness).iter().any(|points| {
                points[1].distance(screen_tip) < 0.01 && points[0].distance(points[1]) < 10.0
            }));
        }

        harness.state_mut().canvas.camera.center = tip;
        harness.run();
        let viewport = harness.get_by_label("Graph canvas").rect();
        let screen_tip = harness.state().canvas.camera.to_screen(tip, viewport);
        let segments = edge_segments(&harness);
        let head = segments
            .iter()
            .filter(|points| {
                points[1].distance(screen_tip) < 0.01 && points[0].distance(points[1]) < 10.0
            })
            .collect::<Vec<_>>();
        assert!(head.iter().any(|points| points[0].y < screen_tip.y));
        assert!(head.iter().any(|points| points[0].y > screen_tip.y));

        let pointer = viewport.center() - Vec2::new(30.0, 0.0);
        pointer_gesture(
            &mut harness,
            pointer,
            pointer,
            PointerButton::Primary,
            Modifiers::NONE,
        );
        assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(0)));
    }
}

#[test]
fn crossing_fragments_outside_the_clip_are_culled_without_changing_picking() {
    use egui::Shape;
    use pautinka_egui::geometry::Camera;

    let mut state = State::default();
    state.graph.nodes = vec![pos2(-2000.0, 0.0), pos2(2000.0, 0.0)];
    state.graph.second_is_loop = false;
    state.graph.edges = [
        EdgePath::try_line(pos2(1500.0, -1000.0), pos2(1500.0, 1000.0)).unwrap(),
        EdgePath::try_line(state.graph.nodes[0], state.graph.nodes[1]).unwrap(),
    ];
    state.canvas.set_labels(false);
    state.canvas.set_camera(Camera {
        center: Pos2::ZERO,
        zoom: 1.0,
        fit: false,
    });
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, state: &mut State| {
                state.canvas.show(ui, &state.graph);
            },
            state,
        );
    harness.run();
    let viewport = harness.get_by_label("Graph canvas").rect();
    let segments = harness
        .output()
        .shapes
        .iter()
        .filter_map(|shape| match shape.shape {
            Shape::LineSegment { points, stroke } if stroke.color == Color32::LIGHT_GREEN => {
                Some(points)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        segments
            .iter()
            .any(|points| points[0].x < viewport.min.x && points[1].x > viewport.max.x)
    );
    assert!(
        segments
            .iter()
            .all(|points| Rect::from_two_pos(points[0], points[1]).intersects(viewport))
    );
    pointer_gesture(
        &mut harness,
        viewport.center(),
        viewport.center(),
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(1)));
}

#[test]
fn flow_direction_changes_arrows_without_rebuilding_routes_or_the_spatial_index() {
    let mut state = State::default();
    state.canvas.set_show_loops(false, &state.graph);
    let mut harness = canvas_harness(state);
    let rebuilds = harness.state().canvas.stats().rebuilds;
    harness.state_mut().graph.direction = pautinka_egui::Direction::Reverse;
    harness.state_mut().graph.revision = harness.state().graph.revision.next_appearance();
    harness.run();
    let viewport = harness.get_by_label("Graph canvas").rect();
    let tip = harness
        .state()
        .canvas
        .camera
        .to_screen(harness.state().graph.edges[0].arrow(true)[1], viewport);
    let heads = harness.output().shapes.iter().filter(|shape| matches!(&shape.shape, egui::Shape::LineSegment { points, .. } if points[1].distance(tip) < 0.001)).count();
    assert_eq!(heads, 2);
    assert_eq!(harness.state().canvas.stats().rebuilds, rebuilds);
}

fn bridge_state() -> State {
    let mut state = State::default();
    state.graph.second_is_loop = false;
    state.graph.direction = pautinka_egui::Direction::Undirected;
    state.graph.nodes.clear();
    state.graph.edges = [
        EdgePath::try_line(pos2(-200.0, 0.0), pos2(200.0, 0.0)).unwrap(),
        EdgePath::try_line(pos2(0.0, -200.0), pos2(0.0, 200.0)).unwrap(),
    ];
    state.canvas.style.crossings.mode = pautinka_egui::CrossingMode::Bridge;
    state.canvas.style.crossings.bridge_height = pautinka_egui::CrossingPoints::new(20.0).unwrap();
    state.canvas.set_camera(pautinka_egui::Camera {
        center: Pos2::ZERO,
        zoom: 1.0,
        fit: false,
    });
    state
}

#[test]
fn bridges_render_above_edges_and_can_be_selected_at_their_apex() {
    let mut harness = canvas_harness(bridge_state());
    for zoom in [1.0, 2.0] {
        harness.state_mut().canvas.camera.zoom = zoom;
        harness.run();
        let center = harness.get_by_label("Graph canvas").rect().center();
        let arcs: Vec<_> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Path(path) if path.points.len() == 25 => Some(path),
                _ => None,
            })
            .collect();
        assert_eq!(arcs.len(), 2);
        assert_eq!(
            arcs[0].stroke.color,
            egui::epaint::ColorMode::Solid(harness.state().canvas.style.background)
        );
        assert_eq!(
            arcs[1].stroke.color,
            egui::epaint::ColorMode::Solid(Color32::LIGHT_GREEN)
        );
        assert!(arcs[1].points[12].distance(center - Vec2::new(0.0, 20.0)) < 0.01);
    }
    let pointer = harness.get_by_label("Graph canvas").rect().center() - Vec2::new(0.0, 20.0);
    pointer_gesture(
        &mut harness,
        pointer,
        pointer,
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Edge(0)));
    let rebuilds = harness.state().canvas.stats().crossing_rebuilds;
    let id = harness.state().graph.edge(1).id;
    harness.state_mut().canvas.set_edge_crossings(
        id,
        Some(pautinka_egui::EdgeCrossings {
            priority: 10,
            options: None,
        }),
    );
    harness.run();
    assert_eq!(harness.state().canvas.stats().crossing_rebuilds, rebuilds);
}

#[test]
fn a_bridge_entering_the_viewport_is_kept_when_its_base_is_outside() {
    let mut state = bridge_state();
    state.canvas.style.crossings.bridge_height = pautinka_egui::CrossingPoints::new(64.0).unwrap();
    let mut harness = canvas_harness(state);
    harness.run();
    let viewport = harness.get_by_label("Graph canvas").rect();
    harness.state_mut().canvas.camera.center.y = -(viewport.height() * 0.5 + 50.0);
    harness.run();
    let arcs: Vec<_> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Path(path) if path.points.len() == 25 => Some(path),
            _ => None,
        })
        .collect();
    assert_eq!(arcs.len(), 2);
    assert!(arcs[1].points[0].y > viewport.bottom());
    assert!(viewport.contains(arcs[1].points[12]));
    let pointer = arcs[1].points[12];
    harness.hover_at(pointer);
    harness.run();
    assert_eq!(
        harness.state().canvas.hovered(),
        Some(pautinka_egui::ElementId::Edge(
            harness.state().graph.edge(0).id
        ))
    );
}
