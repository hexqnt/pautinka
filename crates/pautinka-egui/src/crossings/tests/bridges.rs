use super::*;

#[test]
fn bridges_keep_screen_dimensions_and_auto_side_when_path_is_reversed() {
    let mut graph = TestGraph::new([
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        [pos2(0.0, -100.0), pos2(0.0, 100.0)],
    ]);
    let options = CrossingOptions {
        mode: CrossingMode::Bridge,
        ..CrossingOptions::default()
    };
    for reversed in [false, true] {
        if reversed {
            graph.paths[0].1 = EdgePath::try_line(pos2(100.0, 0.0), pos2(-100.0, 0.0)).unwrap();
            graph.revision = graph.revision.next_geometry();
        }
        let cache = prepare(&graph);
        for zoom in [0.5, 1.0, 4.0] {
            let parts = decorated(&cache, &graph, 0, zoom, options, &BTreeMap::new());
            let arc = &parts.iter().find(|(bridge, _)| *bridge).unwrap().1;
            assert_eq!(arc.len(), 25);
            assert!((arc[0].distance(arc[24]) * zoom - 10.0).abs() < 0.001);
            assert!((arc[12].y * zoom + 6.0).abs() < 0.001);
            assert_eq!(
                decorated(&cache, &graph, 1, zoom, options, &BTreeMap::new()).len(),
                1
            );
        }
    }
    let left = bridge_points(pos2(-5.0, 0.0), pos2(5.0, 0.0), 6.0, BridgeSide::Left);
    let right = bridge_points(pos2(-5.0, 0.0), pos2(5.0, 0.0), 6.0, BridgeSide::Right);
    assert_eq!(left[12], pos2(0.0, -6.0));
    assert_eq!(right[12], pos2(0.0, 6.0));
}

#[test]
fn bridge_sampling_preserves_weighted_interpolation_rounding() {
    fn reference(a: Pos2, b: Pos2, height: f32, side: BridgeSide) -> [Pos2; 25] {
        let delta = b - a;
        let mut normal = delta.normalized().rot90();
        match side {
            BridgeSide::Left => {}
            BridgeSide::Right => normal = -normal,
            BridgeSide::Auto => {
                if (delta.x.abs() >= delta.y.abs() && normal.y > 0.0)
                    || (delta.x.abs() < delta.y.abs() && normal.x < 0.0)
                {
                    normal = -normal;
                }
            }
        }
        let top = a.lerp(b, 0.5) + normal * height;
        let handle = delta * 0.25;
        let curves = [
            [a, a + handle, top - handle, top],
            [top, top + handle, b - handle, b],
        ];
        std::array::from_fn(|index| {
            let (mut curve, step) = if index <= 12 {
                (curves[0], index)
            } else {
                (curves[1], index - 12)
            };
            let t = step as f32 / 12.0;
            // Эталон де Кастельжо использует исходную интерполяцию egui и тот же порядок округлений.
            for remaining in (1..4).rev() {
                for index in 0..remaining {
                    curve[index] = curve[index].lerp(curve[index + 1], t);
                }
            }
            curve[0]
        })
    }

    for scale in [1e-25, 0.0001, 1.0, 10000.0, 1e25] {
        for scale in [scale, -scale] {
            for [a, b] in [
                [pos2(-5.25, 0.0), pos2(5.75, 0.0)],
                [pos2(0.0, -5.25), pos2(0.0, 5.75)],
                [pos2(-3.2, -11.3), pos2(6.1, 22.9)],
                [pos2(-5.0, 5.0), pos2(5.0, -5.0)],
            ] {
                for (a, b) in [(a, b), (b, a)] {
                    for side in [BridgeSide::Auto, BridgeSide::Left, BridgeSide::Right] {
                        let a = a * scale;
                        let b = b * scale;
                        for height in [0.0, 6.0 * scale.abs()] {
                            let actual = bridge_points(a, b, height, side);
                            let expected = reference(a, b, height, side);
                            let bits = |point: Pos2| [point.x.to_bits(), point.y.to_bits()];
                            assert_eq!(
                                actual.map(bits),
                                expected.map(bits),
                                "scale={scale}, a={a:?}, b={b:?}, height={height}, side={side:?}",
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn crowded_bridges_keep_upper_path_and_gap_lower_edges() {
    let graph = TestGraph::new([
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        [pos2(-2.0, -100.0), pos2(-2.0, 100.0)],
        [pos2(2.0, -100.0), pos2(2.0, 100.0)],
    ]);
    let options = CrossingOptions {
        mode: CrossingMode::Bridge,
        ..CrossingOptions::default()
    };
    let parts = decorated(&prepare(&graph), &graph, 0, 1.0, options, &BTreeMap::new());
    assert_eq!(parts.len(), 1);
    assert!(parts.iter().all(|(bridge, _)| !bridge));
    let cache = prepare(&graph);
    for lower in [1, 2] {
        let parts = decorated(&cache, &graph, lower, 1.0, options, &BTreeMap::new());
        assert_eq!(parts.len(), 2);
        assert!(parts.iter().all(|(bridge, _)| !bridge));
        assert!((parts[0].1.last().unwrap().distance(parts[1].1[0]) - 10.0).abs() < 0.001);
    }
}

#[test]
fn bridge_picking_before_preparation_tracks_ids_and_reset_clears_overrides() {
    let mut graph = TestGraph::new([
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        [pos2(0.0, -100.0), pos2(0.0, 100.0)],
    ]);
    let mut canvas = crate::Canvas::default();
    canvas.style.crossings.mode = CrossingMode::Bridge;
    canvas.style.crossings.bridge_height = CrossingPoints::new(20.0).unwrap();
    assert_eq!(
        canvas.pick(pos2(0.0, -20.0), &graph),
        Some(crate::Selection::Edge(0))
    );
    let id = graph.edge(1).id;
    canvas.set_edge_crossings(
        id,
        Some(EdgeCrossings {
            priority: 10,
            options: None,
        }),
    );
    assert_eq!(
        canvas.pick(pos2(20.0, 0.0), &graph),
        Some(crate::Selection::Edge(1))
    );
    graph.paths.swap(0, 1);
    graph.revision = graph.revision.next_topology();
    assert_eq!(
        canvas.pick(pos2(20.0, 0.0), &graph),
        Some(crate::Selection::Edge(0))
    );
    canvas.reset();
    assert_eq!(canvas.edge_crossings(id), None);
    assert_eq!(canvas.style.crossings.mode, CrossingMode::Bridge);
}

#[test]
fn zero_height_bridges_preserve_priority_and_gap_the_lower_edge() {
    let graph = TestGraph::new([
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        [pos2(0.0, -100.0), pos2(0.0, 100.0)],
    ]);
    let cache = prepare(&graph);
    let options = CrossingOptions {
        mode: CrossingMode::Bridge,
        bridge_height: CrossingPoints::new(0.0).unwrap(),
        ..CrossingOptions::default()
    };
    let mut overrides = BTreeMap::new();
    for (upper, lower) in [(0, 1), (1, 0)] {
        overrides.insert(
            graph.edge(upper).id,
            EdgeCrossings {
                priority: 10,
                options: None,
            },
        );
        let upper_parts = decorated(&cache, &graph, upper, 1.0, options, &overrides);
        assert_eq!(
            upper_parts,
            [(false, graph.edge(upper).path.points().to_vec())]
        );
        let lower_parts = decorated(&cache, &graph, lower, 1.0, options, &overrides);
        assert_eq!(lower_parts.len(), 2);
        assert!(lower_parts.iter().all(|(bridge, _)| !bridge));
        assert!(
            (lower_parts[0]
                .1
                .last()
                .unwrap()
                .distance(lower_parts[1].1[0])
                - 10.0)
                .abs()
                < 0.001
        );
        overrides.clear();
    }
}

#[test]
fn bridge_segment_queries_match_the_full_path_and_indexed_picking() {
    let mut graph = TestGraph::new([
        [pos2(-100.0, -100.0), pos2(100.0, -100.0)],
        [pos2(0.0, -200.0), pos2(0.0, 200.0)],
        [pos2(-200.0, 0.0), pos2(200.0, 0.0)],
    ]);
    graph.paths[0].1 = EdgePath::try_polyline([
        pos2(-100.0, -100.0),
        pos2(100.0, -100.0),
        pos2(100.0, 100.0),
        pos2(-100.0, 100.0),
        pos2(-100.0, 200.0),
    ])
    .unwrap();
    let cache = prepare(&graph);
    let overrides = BTreeMap::new();
    let options = CrossingOptions {
        mode: CrossingMode::Bridge,
        ..CrossingOptions::default()
    };
    let view = CrossingView {
        graph: &graph,
        defaults: options,
        overrides: &overrides,
        zoom: 1.0,
        width: &|_| 1.6,
    };
    let mut all = Vec::new();
    cache.bridges(0, None, &view, |points| all.push(points.to_vec()));
    assert_eq!(all.len(), 3);
    let mut by_segment = Vec::new();
    for segment in 0..graph.edge(0).path.points().len() {
        cache.bridges(0, Some(segment), &view, |points| {
            by_segment.push(points.to_vec());
        });
    }
    assert_eq!(all, by_segment);
    let mut canvas = crate::Canvas::default();
    canvas.style.crossings = options;
    canvas.prepare(&graph).unwrap();
    for arc in all {
        assert_eq!(
            canvas.pick(arc[12], &graph),
            Some(crate::Selection::Edge(0))
        );
    }
}

#[test]
fn bridge_endpoints_on_rounding_follow_the_same_removed_interval() {
    let mut graph = TestGraph::new([
        [pos2(-100.0, 0.0), pos2(0.0, 100.0)],
        [pos2(-10.0, -100.0), pos2(-10.0, 200.0)],
    ]);
    graph.paths[0].1 = EdgePath::try_polyline([pos2(-100.0, 0.0), Pos2::ZERO, pos2(0.0, 100.0)])
        .unwrap()
        .with_corner_radius(crate::CornerRadius::try_from(40.0).unwrap())
        .unwrap();
    let parts = decorated(
        &prepare(&graph),
        &graph,
        0,
        1.0,
        CrossingOptions {
            mode: CrossingMode::Bridge,
            ..CrossingOptions::default()
        },
        &BTreeMap::new(),
    );
    let lines: Vec<_> = parts.iter().filter(|(bridge, _)| !bridge).collect();
    let bridges: Vec<_> = parts.iter().filter(|(bridge, _)| *bridge).collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(bridges.len(), 1);
    assert!(lines[0].1.last().unwrap().distance(bridges[0].1[0]) < 0.001);
    assert!(lines[1].1[0].distance(*bridges[0].1.last().unwrap()) < 0.001);
}
