use crate::geometry::Curve;

use super::*;

fn paths(curve: Curve<'_>) -> EdgePath {
    match curve {
        Curve::Quadratic(points) => EdgePath::try_quadratic(points).unwrap(),
        Curve::Cubic(points) => EdgePath::try_cubic(points).unwrap(),
        Curve::Polyline(points) => EdgePath::try_polyline(points).unwrap(),
    }
}

fn arched(cubic: bool, scale: f32, reverse: bool) -> EdgePath {
    let p = |x, y| pos2(x * scale, y * scale);
    if cubic {
        let mut points = [
            p(-100.0, 40.0),
            p(-100.0 / 3.0, -40.0 / 3.0),
            p(100.0 / 3.0, -40.0 / 3.0),
            p(100.0, 40.0),
        ];
        if reverse {
            points.reverse();
        }
        EdgePath::try_cubic(points).unwrap()
    } else {
        let mut points = [p(-100.0, 40.0), p(0.0, -40.0), p(100.0, 40.0)];
        if reverse {
            points.reverse();
        }
        EdgePath::try_quadratic(points).unwrap()
    }
}

#[test]
fn bezier_crossings_refine_positions_and_split_exact_curves_across_sample_vertices() {
    for cubic in [false, true] {
        for reverse in [false, true] {
            for x in [0.0, 13.37] {
                for (scale, nodes) in [(1e-10, 0), (1.0, 0), (1.0, 100), (1e10, 0)] {
                    let mut graph = TestGraph::new([
                        [
                            pos2(x * scale, -100.0 * scale),
                            pos2(x * scale, 100.0 * scale),
                        ],
                        [Pos2::ZERO; 2],
                    ]);
                    graph.nodes = nodes;
                    graph.paths[1].1 = arched(cubic, scale, reverse);
                    let cache = prepare(&graph);
                    assert_eq!(
                        cache.crossings.len(),
                        2,
                        "cubic={cubic}, reverse={reverse}, x={x}, scale={scale}"
                    );
                    let crossing = cache.crossings.iter().find(|hit| hit.edge == 1).unwrap();
                    let position = graph.paths[1]
                        .1
                        .at_distance(crossing.distance)
                        .unwrap()
                        .position
                        / scale;
                    assert!(
                        position.distance(pos2(x, 0.004 * x * x)) < 0.0001,
                        "{position:?}"
                    );
                    let view = CrossingView {
                        graph: &graph,
                        defaults: CrossingOptions::default(),
                        overrides: &BTreeMap::new(),
                        zoom: 1.0 / scale,
                        width: &|_| 2.0,
                    };
                    let mut pieces = Vec::new();
                    cache.curves(1, &view, |curve| pieces.push(paths(curve)));
                    assert_eq!(pieces.len(), 2);
                    assert_eq!(pieces[0].points()[0], graph.paths[1].1.points()[0]);
                    assert_eq!(pieces[1].points().last(), graph.paths[1].1.points().last());
                    let half = 5.0 * scale / crossing.sine;
                    for (piece, distance, last) in [
                        (&pieces[0], crossing.distance - half, true),
                        (&pieces[1], crossing.distance + half, false),
                    ] {
                        assert_eq!(matches!(piece.curve(), Curve::Cubic(_)), cubic);
                        let points = piece.points();
                        let endpoint = if last {
                            points[points.len() - 1]
                        } else {
                            points[0]
                        };
                        let expected = graph.paths[1].1.at_distance(distance).unwrap().position;
                        assert!((endpoint / scale).distance(expected / scale) < 0.0001);
                    }
                }
            }
        }
    }
}

#[test]
fn curve_bridges_match_removed_sections_and_spatial_chunk_queries() {
    for cubic in [false, true] {
        for extent in [100.0, 1000.0] {
            let mut graph =
                TestGraph::new([[Pos2::ZERO; 2], [pos2(13.37, -extent), pos2(13.37, extent)]]);
            graph.paths[0].1 = arched(cubic, extent / 100.0, false);
            let cache = prepare(&graph);
            let view = CrossingView {
                graph: &graph,
                defaults: CrossingOptions {
                    mode: CrossingMode::Bridge,
                    ..CrossingOptions::default()
                },
                overrides: &BTreeMap::new(),
                zoom: 1.0,
                width: &|_| 2.0,
            };
            let mut sections = Vec::new();
            cache.curves(0, &view, |curve| sections.push(paths(curve)));
            assert_eq!(sections.len(), 2);
            let mut bridges = Vec::new();
            cache.bridges(0, None, &view, |points| bridges.push(points.to_vec()));
            assert_eq!(bridges.len(), 1);
            assert!(bridges[0][0].distance(*sections[0].points().last().unwrap()) < 0.001);
            assert!(bridges[0][24].distance(sections[1].points()[0]) < 0.001);
            let mut indexed = Vec::new();
            for chunk in 0..graph.paths[0].1.spatial_chunks().len() {
                cache.bridges(0, Some(chunk), &view, |points| {
                    indexed.push(points.to_vec());
                });
            }
            assert_eq!(indexed, bridges);
            let mut canvas = crate::Canvas::default();
            canvas.style.crossings = view.defaults;
            let apex = bridges[0][12];
            assert_eq!(canvas.pick(apex, &graph), Some(crate::Selection::Edge(0)));
            canvas.prepare(&graph).unwrap();
            assert_eq!(canvas.pick(apex, &graph), Some(crate::Selection::Edge(0)));
        }
    }
}

#[test]
fn quadratic_and_cubic_crossings_use_the_same_cache_in_both_scan_modes() {
    let mut graph = TestGraph::new([[Pos2::ZERO; 2]; 2]);
    graph.paths[0].1 = arched(false, 1.0, false);
    graph.paths[1].1 = EdgePath::try_cubic([
        pos2(-40.0, -100.0),
        pos2(40.0 / 3.0, -100.0 / 3.0),
        pos2(40.0 / 3.0, 100.0 / 3.0),
        pos2(-40.0, 100.0),
    ])
    .unwrap();
    for nodes in [0, 100] {
        graph.nodes = nodes;
        let cache = prepare(&graph);
        assert_eq!(cache.crossings.len(), 2);
        for hit in &cache.crossings {
            assert!((hit.sine - 1.0).abs() < 1e-6);
            assert!(
                graph
                    .edge(hit.edge)
                    .path
                    .at_distance(hit.distance)
                    .unwrap()
                    .position
                    .distance(Pos2::ZERO)
                    < 0.0001
            );
        }
    }
}
