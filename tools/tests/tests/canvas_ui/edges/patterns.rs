use super::*;
use egui::Stroke;
use pautinka_egui::{DefaultRenderer, EdgeStyle, ElementState, LinePattern, Renderer, Style};

struct PatternRenderer(LinePattern);
impl Renderer for PatternRenderer {
    fn edge_style(
        &self,
        index: usize,
        edge: Edge<'_>,
        state: ElementState,
        style: &Style,
    ) -> EdgeStyle {
        EdgeStyle {
            stroke: Stroke::new(2.0, edge.color),
            halo: Some(Stroke::new(7.0, Color32::RED)),
            pattern: self.0,
            ..DefaultRenderer.edge_style(index, edge, state, style)
        }
    }
}

#[test]
fn edge_patterns_keep_matching_halo_geometry_and_paint_order() {
    use egui::{
        Shape,
        epaint::{ColorMode, PathStroke},
    };
    use pautinka_egui::Direction;

    fn without_stroke(shape: &Shape) -> Option<(Color32, Shape)> {
        let mut shape = shape.clone();
        let color = if let Shape::LineSegment { stroke, .. } = &mut shape {
            let color = stroke.color;
            *stroke = Stroke::NONE;
            color
        } else if let Shape::Circle(circle) = &mut shape {
            let color = circle.fill;
            circle.fill = Color32::TRANSPARENT;
            circle.radius = 0.0;
            color
        } else {
            let stroke = match &mut shape {
                Shape::Path(path) => &mut path.stroke,
                Shape::CubicBezier(curve) => &mut curve.stroke,
                Shape::QuadraticBezier(curve) => &mut curve.stroke,
                _ => return None,
            };
            let ColorMode::Solid(color) = stroke.color else {
                return None;
            };
            *stroke = PathStroke::NONE;
            color
        };
        matches!(color, Color32::RED | Color32::LIGHT_GREEN).then_some((color, shape))
    }

    for curve in 0..3 {
        for pattern in [
            LinePattern::Solid,
            LinePattern::Dashed {
                dash: 8.0,
                gap: 5.0,
            },
            LinePattern::Dotted { gap: 5.0 },
            LinePattern::DashDot {
                dash: 8.0,
                gap: 5.0,
            },
            LinePattern::DashDotDot {
                dash: 8.0,
                gap: 5.0,
            },
            LinePattern::Dotted { gap: f32::NAN },
            LinePattern::DashDot {
                dash: 0.0,
                gap: 5.0,
            },
            LinePattern::DashDotDot {
                dash: 8.0,
                gap: -1.0,
            },
            LinePattern::Dashed {
                dash: f32::NAN,
                gap: 5.0,
            },
            LinePattern::Dashed {
                dash: 8.0,
                gap: 0.0,
            },
        ] {
            let mut state = State::default();
            let [from, to] = [state.graph.nodes[0], state.graph.nodes[1]];
            state.graph.edges[0] = match curve {
                0 => EdgePath::try_line(from, to).unwrap(),
                1 => EdgePath::try_quadratic([from, pos2(0.0, -100.0), to]).unwrap(),
                _ => {
                    EdgePath::try_cubic([from, pos2(-50.0, -100.0), pos2(50.0, 100.0), to]).unwrap()
                }
            };
            state.graph.direction = Direction::Both;
            state.canvas.set_labels(false);
            state.canvas.options.lod.enabled = false;
            state.canvas.set_show_loops(false, &state.graph);
            let renderer = PatternRenderer(pattern);
            let harness = Harness::builder()
                .with_size(Vec2::new(800.0, 600.0))
                .build_ui_state(
                    move |ui, state: &mut State| {
                        state.canvas.show_with(ui, &state.graph, &renderer);
                    },
                    state,
                );
            let shapes: Vec<_> = harness
                .output()
                .shapes
                .iter()
                .filter_map(|shape| without_stroke(&shape.shape))
                .collect();
            assert_ne!(shapes, []);
            let (halo, body) = shapes.split_at(shapes.len() / 2);
            assert!(halo.iter().all(|(color, _)| *color == Color32::RED));
            assert!(body.iter().all(|(color, _)| *color == Color32::LIGHT_GREEN));
            assert!(
                halo.iter()
                    .map(|(_, shape)| shape)
                    .eq(body.iter().map(|(_, shape)| shape))
            );
            let (path, arrows) = body.split_at(body.len() - 4);
            assert!(
                arrows
                    .iter()
                    .all(|(_, shape)| matches!(shape, Shape::LineSegment { .. }))
            );
            match pattern {
                LinePattern::Dotted { gap } if gap.is_finite() => {
                    assert!(path.len() > 4);
                    assert!(
                        path.iter()
                            .all(|(_, shape)| matches!(shape, Shape::Circle(_)))
                    );
                }
                LinePattern::DashDot { dash, gap } | LinePattern::DashDotDot { dash, gap }
                    if dash > 0.0 && gap > 0.0 =>
                {
                    assert!(
                        path.iter()
                            .any(|(_, shape)| matches!(shape, Shape::Circle(_)))
                    );
                    assert!(
                        path.iter()
                            .any(|(_, shape)| matches!(shape, Shape::LineSegment { .. }))
                    );
                }
                LinePattern::Dashed { dash, gap } if dash.is_finite() && gap > 0.0 => {
                    assert!(path.len() > 4);
                    assert!(
                        path.iter()
                            .all(|(_, shape)| matches!(shape, Shape::LineSegment { .. }))
                    );
                }
                _ => {
                    assert_eq!(path.len(), 1);
                    assert!(matches!(
                        (curve, &path[0].1),
                        (0, Shape::LineSegment { .. })
                            | (1, Shape::QuadraticBezier(_))
                            | (2, Shape::CubicBezier(_))
                    ));
                }
            }
        }
    }
}

#[test]
fn dot_sizes_and_intervals_stay_constant_with_zoom_and_bridge_arcs_use_the_pattern() {
    use pautinka_egui::CrossingMode;

    for pattern in [
        LinePattern::Dotted { gap: 5.0 },
        LinePattern::DashDot {
            dash: 8.0,
            gap: 5.0,
        },
        LinePattern::DashDotDot {
            dash: 8.0,
            gap: 5.0,
        },
    ] {
        let mut state = bridge_state();
        state.canvas.set_labels(false);
        state.canvas.options.lod.enabled = false;
        let renderer = PatternRenderer(pattern);
        let mut harness = Harness::builder()
            .with_size(Vec2::new(800.0, 600.0))
            .build_ui_state(
                move |ui, state: &mut State| {
                    state.canvas.show_with(ui, &state.graph, &renderer);
                },
                state,
            );
        for zoom in [1.0, 2.0] {
            for mode in [CrossingMode::None, CrossingMode::Bridge] {
                harness.state_mut().canvas.camera.zoom = zoom;
                harness.state_mut().canvas.style.crossings.mode = mode;
                harness.run();
                let center = harness.get_by_label("Graph canvas").rect().center();
                let dots: Vec<_> = harness
                    .output()
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Circle(circle) if circle.fill == Color32::LIGHT_GREEN => {
                            assert_eq!(circle.radius, 1.0);
                            Some(circle.center)
                        }
                        egui::Shape::Circle(circle) if circle.fill == Color32::RED => {
                            assert_eq!(circle.radius, 3.5);
                            None
                        }
                        _ => None,
                    })
                    .collect();
                assert!(dots.len() > 4);
                if mode == CrossingMode::Bridge {
                    assert!(dots.iter().any(|point| (point.x - center.x).abs() > 0.1
                        && (point.y - center.y).abs() > 0.1));
                } else {
                    let horizontal: Vec<_> = dots
                        .iter()
                        .filter(|point| {
                            (point.y - center.y).abs() < 0.01 && point.x < center.x - 30.0
                        })
                        .collect();
                    assert!(horizontal.len() > 4);
                    for pair in horizontal.windows(2) {
                        let spacing = pair[1].x - pair[0].x;
                        let matches = match pattern {
                            LinePattern::Dotted { .. } => (spacing - 7.0).abs() < 0.001,
                            LinePattern::DashDot { .. } => (spacing - 20.0).abs() < 0.001,
                            LinePattern::DashDotDot { .. } => {
                                (spacing - 7.0).abs() < 0.001 || (spacing - 20.0).abs() < 0.001
                            }
                            _ => unreachable!(),
                        };
                        assert!(
                            matches,
                            "Unexpected dot spacing: {spacing}; pattern={pattern:?}, zoom={zoom}"
                        );
                    }
                }
            }
        }
    }
}
