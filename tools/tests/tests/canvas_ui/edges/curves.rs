use super::*;

#[test]
fn bezier_gaps_and_bridges_preserve_curves_and_support_pointer_selection() {
    use pautinka_egui::CrossingMode;

    for cubic in [false, true] {
        for mode in [CrossingMode::Gap, CrossingMode::Bridge] {
            let mut state = bridge_state();
            let curve = if cubic {
                EdgePath::try_cubic([
                    pos2(-100.0, 40.0),
                    pos2(-100.0 / 3.0, -40.0 / 3.0),
                    pos2(100.0 / 3.0, -40.0 / 3.0),
                    pos2(100.0, 40.0),
                ])
                .unwrap()
            } else {
                EdgePath::try_quadratic([pos2(-100.0, 40.0), pos2(0.0, -40.0), pos2(100.0, 40.0)])
                    .unwrap()
            };
            let index = usize::from(mode == CrossingMode::Gap);
            if index == 1 {
                state.graph.edges.swap(0, 1);
            }
            state.graph.edges[index] = curve;
            state.canvas.style.crossings.mode = mode;
            let mut harness = canvas_harness(state);
            let mut pointer = Pos2::ZERO;
            for zoom in [1.0, 2.0] {
                harness.state_mut().canvas.camera.zoom = zoom;
                harness.run();
                let mut endpoints = harness
                    .output()
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::QuadraticBezier(curve) => {
                            Some([curve.points[0], curve.points[2]])
                        }
                        egui::Shape::CubicBezier(curve) => Some([curve.points[0], curve.points[3]]),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                endpoints.sort_by(|a, b| a[0].x.total_cmp(&b[0].x));
                assert_eq!(endpoints.len(), 2);
                assert!((endpoints[1][0].x - endpoints[0][1].x - 10.0).abs() < 0.02);
                let center = harness.get_by_label("Graph canvas").rect().center();
                if mode == CrossingMode::Bridge {
                    let arcs = harness
                        .output()
                        .shapes
                        .iter()
                        .filter_map(|shape| match &shape.shape {
                            egui::Shape::Path(path) if path.points.len() == 25 => Some(path),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(arcs.len(), 2);
                    assert!(arcs[1].points[0].distance(endpoints[0][1]) < 0.001);
                    assert!(arcs[1].points[24].distance(endpoints[1][0]) < 0.001);
                    assert!((arcs[1].points[12].y - center.y + 20.0).abs() < 0.11);
                    pointer = arcs[1].points[12];
                } else {
                    pointer = center + Vec2::new(2.0, 0.0);
                }
            }
            pointer_gesture(
                &mut harness,
                pointer,
                pointer,
                egui::PointerButton::Primary,
                egui::Modifiers::NONE,
            );
            assert_eq!(
                harness.state().canvas.selected(),
                Some(Selection::Edge(index))
            );
        }
    }
}
