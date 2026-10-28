use std::collections::BTreeSet;

use super::*;

#[test]
fn crossings_reuse_spatial_changes_and_read_history_after_skipped_updates() {
    let mut graph = TestGraph::crossing();
    let mut spatial = SpatialCache::default();
    let mut cache = CrossingCache::default();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    for (step, x) in [20.0, 40.0, 60.0].into_iter().enumerate() {
        let before = graph.revision;
        graph.paths[0].1 = EdgePath::try_line(pos2(x, -100.0), pos2(x, 100.0)).unwrap();
        graph.revision = before.next_geometry();
        graph
            .changes
            .record(before, graph.revision, [crate::Selection::Edge(0); 2]);
        spatial.synchronize(&graph).unwrap();
        let reads = graph.journal_reads.get();
        if step != 1 {
            cache.prepare(&graph, &spatial);
            assert_eq!(graph.journal_reads.get(), reads + usize::from(step == 2));
            let full = prepare(&graph);
            for index in 0..graph.edge_count() {
                assert_eq!(
                    pieces(&cache, &graph, index, 1.0, 10.0),
                    pieces(&full, &graph, index, 1.0, 10.0),
                );
            }
        }
        graph.revision = graph.revision.next_appearance();
        spatial.synchronize(&graph).unwrap();
    }
    assert_eq!(cache.full_rebuilds, 1);

    graph.paths[0].1 = EdgePath::try_line(pos2(200.0, -100.0), pos2(200.0, 100.0)).unwrap();
    graph.revision = graph.revision.next_geometry();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    assert!(cache.crossings.is_empty());
    assert_eq!(cache.full_rebuilds, 2);
}

#[test]
fn local_crossing_updates_match_full_rebuilds_for_both_edge_orders_and_curve_changes() {
    let mut graph = TestGraph::new([
        [pos2(0.0, -100.0), pos2(0.0, 100.0)],
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        [pos2(50.0, -100.0), pos2(50.0, 100.0)],
        [pos2(-100.0, 30.0), pos2(100.0, 30.0)],
    ]);
    graph.paths[1].1 = EdgePath::try_polyline([
        pos2(-100.0, -37.2),
        pos2(-100.0, 0.0),
        pos2(100.0, 0.0),
        pos2(100.0, 65.3),
    ])
    .unwrap();
    let mut spatial = SpatialCache::default();
    spatial.synchronize(&graph).unwrap();
    let mut cache = prepare(&graph);
    for (edges, paths) in [
        (
            vec![0],
            vec![EdgePath::try_line(pos2(40.0, -100.0), pos2(40.0, 100.0)).unwrap()],
        ),
        (
            vec![0],
            vec![EdgePath::try_line(pos2(200.0, -100.0), pos2(200.0, 100.0)).unwrap()],
        ),
        (
            vec![1],
            vec![EdgePath::try_line(pos2(-100.0, 10.0), pos2(250.0, 10.0)).unwrap()],
        ),
        (
            vec![0, 1],
            vec![
                EdgePath::try_line(pos2(40.0, -100.0), pos2(40.0, 100.0)).unwrap(),
                EdgePath::try_line(pos2(-100.0, 0.0), pos2(100.0, 0.0)).unwrap(),
            ],
        ),
        (
            vec![2],
            vec![
                EdgePath::try_quadratic([pos2(0.0, 0.0), pos2(100.0, -200.0), pos2(200.0, 0.0)])
                    .unwrap(),
            ],
        ),
        (
            vec![2],
            vec![EdgePath::try_line(pos2(-20.0, -100.0), pos2(-20.0, 100.0)).unwrap()],
        ),
    ] {
        let before = graph.revision;
        for (&index, path) in edges.iter().zip(paths) {
            graph.paths[index].1 = path;
        }
        graph.revision = before.next_geometry();
        graph.changes.record(
            before,
            graph.revision,
            edges.into_iter().map(crate::Selection::Edge),
        );
        spatial.synchronize(&graph).unwrap();
        cache.prepare(&graph, &spatial);
        let full = prepare(&graph);
        for edge in 0..graph.edge_count() {
            assert_eq!(
                pieces(&cache, &graph, edge, 1.0, 10.0),
                pieces(&full, &graph, edge, 1.0, 10.0)
            );
        }
        assert_eq!(cache.full_rebuilds, 1);
    }
    assert!(cache.scanned_edges < graph.edge_count() as u64 * 7);
    graph.paths.swap(0, 1);
    graph.revision = graph.revision.next_topology();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    assert_eq!(cache.full_rebuilds, 2);
}

#[test]
fn spatial_crossing_candidates_and_updates_match_all_segment_pairs() {
    let mut seed = 42_u64;
    let mut coordinate = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 32) as u32 as f32 / u32::MAX as f32 * 400.0 - 200.0
    };
    for case in 0..8 {
        let mut graph = TestGraph::new([
            [pos2(0.0, 0.0), pos2(200.0, 200.0)],
            [pos2(0.0, 150.0), pos2(20.0, 200.0)],
            [pos2(-100.0, 0.0), pos2(0.0, 100.0)],
            [pos2(0.0, -100.0), pos2(0.0, 200.0)],
        ]);
        graph.nodes = case % 2 * 48;
        graph.paths[2].1 =
            EdgePath::try_polyline([pos2(-100.0, 0.0), Pos2::ZERO, pos2(0.0, 100.0)])
                .unwrap()
                .with_corner_radius(crate::CornerRadius::try_from(40.0).unwrap())
                .unwrap();
        let x = graph.paths[2].1.points()[9].x;
        graph.paths[3].1 = EdgePath::try_line(pos2(x, -100.0), pos2(x, 200.0)).unwrap();
        for index in 4..24 {
            let path = EdgePath::try_polyline([
                pos2(coordinate(), coordinate()),
                pos2(coordinate(), coordinate()),
                pos2(coordinate(), coordinate()),
            ])
            .unwrap()
            .with_corner_radius(crate::CornerRadius::try_from((index % 3) as f32 * 12.0).unwrap())
            .unwrap();
            graph.paths.push((EdgeId::new(0), path));
        }
        for (index, (id, _)) in graph.paths.iter_mut().enumerate() {
            *id = EdgeId::new(((index * 13 + 7) % 24) as u64);
        }
        let mut spatial = SpatialCache::default();
        let mut cache = CrossingCache::default();
        for step in 0..3 {
            if step != 0 {
                let before = graph.revision;
                let changed = (0..graph.edge_count())
                    .step_by(if step == 1 { 3 } else { 1 })
                    .collect::<Vec<_>>();
                for &index in &changed {
                    graph.paths[index].1 = EdgePath::try_polyline([
                        pos2(coordinate(), coordinate()),
                        pos2(coordinate(), coordinate()),
                        pos2(coordinate(), coordinate()),
                    ])
                    .unwrap()
                    .with_corner_radius(
                        crate::CornerRadius::try_from((index % 3) as f32 * 12.0).unwrap(),
                    )
                    .unwrap();
                }
                graph.revision = before.next_geometry();
                graph.changes.record(
                    before,
                    graph.revision,
                    changed.into_iter().map(crate::Selection::Edge),
                );
            }
            spatial.synchronize(&graph).unwrap();
            cache.prepare(&graph, &spatial);
            let mut expected = BTreeSet::new();
            for (first, (_, path)) in graph.paths.iter().enumerate() {
                for (second, (_, other)) in graph.paths.iter().enumerate().skip(first + 1) {
                    for (segment, pair) in path.points().windows(2).enumerate() {
                        for (other_segment, other_pair) in other.points().windows(2).enumerate() {
                            if intersection(
                                [pair[0], pair[1]],
                                [other_pair[0], other_pair[1]],
                                [
                                    path.is_smooth_vertex(segment),
                                    other.is_smooth_vertex(other_segment),
                                ],
                            )
                            .is_some()
                            {
                                expected.insert((first, second, segment, other_segment));
                                expected.insert((second, first, other_segment, segment));
                            }
                        }
                    }
                }
            }
            let actual = cache
                .crossings
                .iter()
                .map(|hit| (hit.edge, hit.other, hit.segment, hit.other_segment))
                .collect::<BTreeSet<_>>();
            assert_eq!(actual.len(), cache.crossings.len());
            assert_eq!(actual, expected);
            assert_eq!(cache.full_rebuilds, 1);

            let full = prepare(&graph);
            for edge in 0..graph.edge_count() {
                let actual = pieces(&cache, &graph, edge, 1.0, 10.0);
                let expected = pieces(&full, &graph, edge, 1.0, 10.0);
                assert_eq!(actual.len(), expected.len());
                for (actual, expected) in actual.iter().zip(&expected) {
                    assert_eq!(actual.len(), expected.len());
                    for (actual, expected) in actual.iter().zip(expected) {
                        assert!(actual.distance(*expected) < 0.001);
                    }
                }
            }
        }
    }
}

#[test]
fn overlapping_nodes_without_two_edges_preserve_whole_paths() {
    for edge_count in 0..=1 {
        let mut graph = TestGraph::new(
            std::iter::once([pos2(-100.0, -100.0), pos2(100.0, 100.0)]).take(edge_count),
        );
        graph.nodes = 512;
        let cache = prepare(&graph);
        assert!(cache.crossings.is_empty());
        for edge in 0..graph.edge_count() {
            assert_eq!(
                pieces(&cache, &graph, edge, 1.0, 10.0),
                [graph.edge(edge).path.points()]
            );
        }
    }
}

#[test]
fn moving_an_upper_edge_refreshes_gaps_and_new_graphs_do_not_reuse_them() {
    let mut graph = TestGraph::crossing();
    let mut spatial = SpatialCache::default();
    let mut cache = CrossingCache::default();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    graph.paths[0].1 = EdgePath::try_line(pos2(200.0, -100.0), pos2(200.0, 100.0)).unwrap();
    graph.revision = graph.revision.next_geometry();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    assert!(cache.crossings.is_empty());
    let other = TestGraph::crossing();
    spatial.synchronize(&other).unwrap();
    cache.prepare(&other, &spatial);
    assert_eq!(cache.crossings.len(), 2);
}

#[test]
fn empty_crossing_ranges_between_edges_preserve_whole_paths() {
    let graph = TestGraph::new([
        [pos2(0.0, -100.0), pos2(0.0, 100.0)],
        [pos2(-100.0, -30.0), pos2(100.0, -30.0)],
        [pos2(300.0, -100.0), pos2(300.0, 100.0)],
        [pos2(-100.0, 30.0), pos2(100.0, 30.0)],
        [pos2(400.0, -100.0), pos2(400.0, 100.0)],
    ]);
    let cache = prepare(&graph);
    for edge in [0, 2, 4] {
        assert_eq!(
            pieces(&cache, &graph, edge, 1.0, 10.0),
            [graph.edge(edge).path.points()]
        );
    }
    for edge in [1, 3] {
        let result = pieces(&cache, &graph, edge, 1.0, 10.0);
        assert_eq!(result.len(), 2);
        assert!((result[0].last().unwrap().distance(result[1][0]) - 10.0).abs() < 0.001);
    }
}

#[test]
fn shrinking_topology_and_replacing_graph_refresh_crossing_ranges() {
    let mut graph = TestGraph::new([
        [pos2(0.0, -100.0), pos2(0.0, 100.0)],
        [pos2(-100.0, 0.0), pos2(100.0, 0.0)],
        [pos2(40.0, -100.0), pos2(40.0, 100.0)],
        [pos2(-100.0, 30.0), pos2(100.0, 30.0)],
    ]);
    let mut spatial = SpatialCache::default();
    let mut cache = CrossingCache::default();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    assert_eq!(pieces(&cache, &graph, 3, 1.0, 10.0).len(), 3);

    graph.paths.truncate(2);
    graph.revision = graph.revision.next_topology();
    spatial.synchronize(&graph).unwrap();
    cache.prepare(&graph, &spatial);
    assert_eq!(cache.offsets.len(), graph.edge_count() + 1);
    assert_eq!(pieces(&cache, &graph, 1, 1.0, 10.0).len(), 2);

    let other = TestGraph::new([[pos2(-100.0, 30.0), pos2(100.0, 30.0)]]);
    spatial.synchronize(&other).unwrap();
    cache.prepare(&other, &spatial);
    assert_eq!(cache.offsets, []);
    assert_eq!(
        pieces(&cache, &other, 0, 1.0, 10.0),
        [other.edge(0).path.points()]
    );
}
