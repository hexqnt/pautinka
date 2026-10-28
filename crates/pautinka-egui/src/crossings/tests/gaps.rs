use super::*;

#[test]
fn proper_crossings_exclude_endpoints_overlaps_and_degenerate_segments() {
    let horizontal = [pos2(-100.0, 0.0), pos2(100.0, 0.0)];
    assert_eq!(
        intersection(
            horizontal,
            [pos2(0.0, -100.0), pos2(0.0, 100.0)],
            [false; 2]
        ),
        Some(Intersection {
            fraction: 0.5,
            other_fraction: 0.5,
            sine: 1.0
        })
    );
    let Intersection { sine, .. } = intersection(
        horizontal,
        [pos2(-100.0, -100.0), pos2(100.0, 100.0)],
        [false; 2],
    )
    .unwrap();
    assert!((sine - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.0001);
    for other in [
        [pos2(0.0, 0.0), pos2(0.0, 100.0)],
        [pos2(-50.0, 0.0), pos2(50.0, 0.0)],
        [Pos2::ZERO; 2],
        [pos2(200.0, -100.0), pos2(200.0, 100.0)],
    ] {
        assert!(intersection(horizontal, other, [false; 2]).is_none());
    }
}

#[test]
fn crossings_and_screen_sized_gaps_are_invariant_under_world_unit_changes() {
    for scale in [1e-25, 0.0001, 1.0, 10000.0, 1e25] {
        let graph = TestGraph::new([
            [pos2(0.0, -100.0), pos2(0.0, 100.0)].map(|p| p * scale),
            [pos2(-100.0, 0.0), pos2(100.0, 0.0)].map(|p| p * scale),
        ]);
        let cache = prepare(&graph);
        let parts = pieces(&cache, &graph, 1, 1.0 / scale, 10.0);
        assert_eq!(parts.len(), 2);
        let gap = parts[1][0] - *parts[0].last().unwrap();
        assert!((gap.x.hypot(gap.y) / scale - 10.0).abs() < 0.001);
    }
}

#[test]
fn gaps_are_screen_sized_and_upper_edge_is_stable_after_reordering() {
    let mut graph = TestGraph::crossing();
    for zoom in [0.5, 1.0, 4.0] {
        let cache = prepare(&graph);
        let result = pieces(&cache, &graph, 1, zoom, 10.0);
        assert_eq!(result.len(), 2);
        assert!((result[0].last().unwrap().distance(result[1][0]) * zoom - 10.0).abs() < 0.001);
        assert_eq!(pieces(&cache, &graph, 0, zoom, 10.0).len(), 1);
        assert_eq!(pieces(&cache, &graph, 1, zoom, 0.0).len(), 1);
    }
    graph.paths.swap(0, 1);
    graph.revision = graph.revision.next_topology();
    let cache = prepare(&graph);
    assert_eq!(pieces(&cache, &graph, 0, 1.0, 10.0).len(), 2);
    assert_eq!(pieces(&cache, &graph, 1, 1.0, 10.0).len(), 1);
}

#[test]
fn nearby_gaps_merge_and_disjoint_gaps_keep_the_middle_piece() {
    for (positions, expected) in [([-20.0, 20.0], 3), ([-2.0, 2.0], 2)] {
        let graph = TestGraph::new([
            [pos2(positions[0], -100.0), pos2(positions[0], 100.0)],
            [pos2(positions[1], -100.0), pos2(positions[1], 100.0)],
            [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        ]);
        let result = pieces(&prepare(&graph), &graph, 2, 1.0, 10.0);
        assert_eq!(result.len(), expected);
        assert!(result.iter().all(|piece| piece.len() >= 2));
        if expected == 3 {
            assert!(result[1][0].distance(pos2(-15.0, 0.0)) < 0.001);
            assert!(result[1][1].distance(pos2(15.0, 0.0)) < 0.001);
        }
    }
}

#[test]
fn bends_are_preserved_and_nearby_corners_are_skipped() {
    let mut graph = TestGraph::crossing();
    graph.paths[1].1 = EdgePath::try_polyline([
        pos2(-100.0, -50.0),
        pos2(-100.0, 0.0),
        pos2(100.0, 0.0),
        pos2(100.0, 50.0),
    ])
    .unwrap();
    let result = pieces(&prepare(&graph), &graph, 1, 1.0, 10.0);
    let expected = [
        [pos2(-100.0, -50.0), pos2(-100.0, 0.0), pos2(-5.0, 0.0)],
        [pos2(5.0, 0.0), pos2(100.0, 0.0), pos2(100.0, 50.0)],
    ];
    assert_eq!(result.len(), expected.len());
    for (actual, expected) in result.iter().zip(expected) {
        assert_eq!(actual.len(), expected.len());
        assert!(
            actual
                .iter()
                .zip(expected)
                .all(|(a, b)| a.distance(b) < 0.001)
        );
    }
    graph.paths[0].1 = EdgePath::try_line(pos2(-98.0, -100.0), pos2(-98.0, 100.0)).unwrap();
    assert_eq!(pieces(&prepare(&graph), &graph, 1, 1.0, 10.0).len(), 1);
}

#[test]
fn arrow_changes_do_not_require_rebuilding_intersections() {
    let mut graph = TestGraph::new([
        [pos2(60.0, -100.0), pos2(60.0, 100.0)],
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
    ]);
    let cache = prepare(&graph);
    assert_eq!(pieces(&cache, &graph, 1, 1.0, 10.0).len(), 2);
    graph.direction = Direction::Forward;
    assert_eq!(pieces(&cache, &graph, 1, 1.0, 10.0).len(), 1);
}

#[test]
fn disabled_gaps_preserve_paths_with_unprepared_or_stale_cache() {
    let mut graph = TestGraph::crossing();
    let empty = CrossingCache::default();
    for gap in [0.0, -1.0, f32::NAN, f32::INFINITY, 10.0] {
        assert_eq!(
            pieces(&empty, &graph, 1, 1.0, gap),
            [graph.edge(1).path.points()]
        );
    }

    let cache = prepare(&graph);
    graph.paths[1].1 = EdgePath::try_line(pos2(-200.0, 0.0), pos2(100.0, 0.0)).unwrap();
    graph.revision = graph.revision.next_geometry();
    assert_eq!(
        pieces(&cache, &graph, 1, 1.0, 0.0),
        [graph.edge(1).path.points()]
    );
}

#[test]
fn crossing_parameters_reject_invalid_external_values() {
    for value in [f32::NAN, f32::INFINITY, -1.0, 128.1] {
        assert!(CrossingPoints::new(value).is_none());
    }
    for value in [f32::NAN, f32::INFINITY, 0.0, 90.1] {
        assert!(CrossingAngle::new(value).is_none());
    }
    for value in [0.0, 10.0, 128.0] {
        assert_eq!(CrossingPoints::new(value).unwrap().get(), value);
    }
}

#[test]
fn appearance_priority_and_overrides_change_without_geometry_rebuilds() {
    let graph = TestGraph::crossing();
    let cache = prepare(&graph);
    let mut options = CrossingOptions::default();
    let mut overrides = BTreeMap::new();
    assert_eq!(
        decorated(&cache, &graph, 1, 1.0, options, &overrides).len(),
        2
    );
    options.order = CrossingOrder::HorizontalFirst;
    assert_eq!(
        decorated(&cache, &graph, 0, 1.0, options, &overrides).len(),
        2
    );
    assert_eq!(
        decorated(&cache, &graph, 1, 1.0, options, &overrides).len(),
        1
    );
    options.order = CrossingOrder::VerticalFirst;
    assert_eq!(
        decorated(&cache, &graph, 1, 1.0, options, &overrides).len(),
        2
    );
    overrides.insert(
        graph.edge(1).id,
        EdgeCrossings {
            priority: 10,
            options: None,
        },
    );
    assert_eq!(
        decorated(&cache, &graph, 0, 1.0, options, &overrides).len(),
        2
    );
    options.mode = CrossingMode::Bridge;
    assert!(
        decorated(&cache, &graph, 1, 1.0, options, &overrides)
            .iter()
            .any(|(bridge, _)| *bridge)
    );
    overrides.get_mut(&graph.edge(1).id).unwrap().options = Some(CrossingOptions {
        mode: CrossingMode::None,
        ..options
    });
    assert_eq!(
        decorated(&cache, &graph, 1, 1.0, options, &overrides).len(),
        1
    );
    assert_eq!(cache.full_rebuilds, 1);
}

#[test]
fn automatic_width_uses_both_strokes_and_acute_crossings_obey_angle_setting() {
    let graph = TestGraph::crossing();
    let options = CrossingOptions {
        size: CrossingSize::Auto {
            padding: CrossingPoints::new(3.0).unwrap(),
        },
        ..CrossingOptions::default()
    };
    let parts = decorated(&prepare(&graph), &graph, 1, 1.0, options, &BTreeMap::new());
    assert!((parts[0].1.last().unwrap().distance(parts[1].1[0]) - 18.0).abs() < 0.001);

    let graph = TestGraph::new([
        [pos2(-100.0, -17.63), pos2(100.0, 17.63)],
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
    ]);
    let cache = prepare(&graph);
    assert_eq!(
        decorated(&cache, &graph, 1, 1.0, options, &BTreeMap::new()).len(),
        1
    );
    let options = CrossingOptions {
        min_angle: CrossingAngle::new(5.0).unwrap(),
        ..options
    };
    assert_eq!(
        decorated(&cache, &graph, 1, 1.0, options, &BTreeMap::new()).len(),
        2
    );
}

#[test]
fn rounded_crossings_span_multiple_segments_and_include_smooth_vertices() {
    for reverse in [false, true] {
        for zoom in [0.5, 1.0, 4.0] {
            let mut points = [pos2(-100.0, 0.0), Pos2::ZERO, pos2(0.0, 100.0)];
            if reverse {
                points.reverse();
            }
            let rounded = EdgePath::try_polyline(points)
                .unwrap()
                .with_corner_radius(crate::CornerRadius::try_from(40.0).unwrap())
                .unwrap();
            let arc = &rounded.points()[1..rounded.points().len() - 1];
            // Пересечение внутри хорды и точно на плавном стыке; длина разрыва больше одной хорды.
            for crossing in [arc[8].lerp(arc[9], 0.5), arc[9]] {
                let mut graph = TestGraph::new([
                    [pos2(crossing.x, -100.0), pos2(crossing.x, 200.0)],
                    [points[0], points[2]],
                ]);
                graph.paths[1].1 = rounded.clone();
                let cache = prepare(&graph);
                assert_eq!(
                    cache.crossings.len(),
                    2,
                    "reverse={reverse}, crossing={crossing:?}"
                );
                let result = pieces(&cache, &graph, 1, zoom, 10.0);
                assert_eq!(result.len(), 2, "reverse={reverse}, zoom={zoom}");
                assert_eq!(result[0].first(), rounded.points().first());
                assert_eq!(result[1].last(), rounded.points().last());
                let remaining: f32 = result
                    .iter()
                    .flat_map(|piece| piece.windows(2))
                    .map(|pair| pair[0].distance(pair[1]))
                    .sum();
                let intersection = cache
                    .crossings
                    .iter()
                    .find(|crossing| crossing.edge == 1)
                    .unwrap();
                let expected_gap = 10.0 / (zoom * intersection.sine);
                assert!((rounded.length() - remaining - expected_gap).abs() < 0.001);
                let before = rounded
                    .at_distance(intersection.distance - expected_gap * 0.5)
                    .unwrap()
                    .position;
                let after = rounded
                    .at_distance(intersection.distance + expected_gap * 0.5)
                    .unwrap()
                    .position;
                assert!(result[0].last().unwrap().distance(before) < 0.001);
                assert!(result[1][0].distance(after) < 0.001);
                assert_eq!(
                    pieces(&cache, &graph, 0, zoom, 10.0),
                    [graph.edge(0).path.points()]
                );
            }
        }
    }
}

#[test]
fn overlapping_gaps_on_different_arc_segments_merge() {
    let rounded = EdgePath::try_polyline([pos2(-100.0, 0.0), Pos2::ZERO, pos2(0.0, 100.0)])
        .unwrap()
        .with_corner_radius(crate::CornerRadius::try_from(40.0).unwrap())
        .unwrap();
    let arc = &rounded.points()[1..rounded.points().len() - 1];
    let x = [arc[8].lerp(arc[9], 0.5).x, arc[9].lerp(arc[10], 0.5).x];
    let mut graph = TestGraph::new([
        [pos2(x[0], -100.0), pos2(x[0], 200.0)],
        [pos2(x[1], -100.0), pos2(x[1], 200.0)],
        [pos2(-100.0, 0.0), pos2(0.0, 100.0)],
    ]);
    graph.paths[2].1 = rounded;
    let cache = prepare(&graph);
    assert_eq!(cache.crossings.len(), 4);
    assert_eq!(pieces(&cache, &graph, 2, 1.0, 10.0).len(), 2);
}
