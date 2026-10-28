use std::cell::Cell;

use egui::{Rect, pos2};

use crate::{CrossingAngle, Direction, EdgePath, GraphId, GraphRevision, Node};

use super::*;

struct TestGraph {
    id: GraphId,
    paths: Vec<EdgePath>,
    direction: Direction,
    reads: Cell<usize>,
}

impl Graph for TestGraph {
    fn id(&self) -> GraphId {
        self.id
    }

    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }

    fn node_count(&self) -> usize {
        0
    }

    fn edge_count(&self) -> usize {
        self.paths.len()
    }

    fn node(&self, _: usize) -> Node<'_> {
        unreachable!()
    }

    fn edge(&self, index: usize) -> Edge<'_> {
        self.reads.set(self.reads.get() + 1);
        Edge {
            direction: self.direction,
            ..Edge::new(EdgeId::new(index as u64), &self.paths[index])
        }
    }

    fn bounds(&self) -> Rect {
        Rect::NOTHING
    }
}

fn intersecting_lines(positions: &[f32], length: f32) -> (TestGraph, CrossingCache) {
    let mut paths = vec![EdgePath::try_line(pos2(0.0, 0.0), pos2(length, 0.0)).unwrap()];
    let mut crossings = Vec::new();
    let mut reverse = Vec::new();
    for (index, &distance) in positions.iter().enumerate() {
        let other = index + 1;
        paths.push(EdgePath::try_line(pos2(distance, -100.0), pos2(distance, 100.0)).unwrap());
        let crossing = Crossing {
            edge: 0,
            other,
            segment: 0,
            fraction: distance / length,
            sine: 1.0,
            horizontal: true,
            other_horizontal: false,
            other_segment: 0,
            other_fraction: 0.5,
            clearance: distance.min(length - distance),
            distance,
        };
        crossings.push(crossing);
        reverse.push(Crossing {
            edge: other,
            other: 0,
            fraction: 0.5,
            horizontal: false,
            other_horizontal: true,
            other_fraction: crossing.fraction,
            clearance: 100.0,
            distance: 100.0,
            ..crossing
        });
    }
    let mut offsets = vec![0, crossings.len()];
    offsets.extend((1..=reverse.len()).map(|count| crossings.len() + count));
    crossings.extend(reverse);
    let graph = TestGraph {
        id: GraphId::fresh(),
        paths,
        direction: Direction::Undirected,
        reads: Cell::new(0),
    };
    let cache = CrossingCache {
        key: Some((graph.id(), graph.revision())),
        crossings,
        offsets,
        ..CrossingCache::default()
    };
    (graph, cache)
}

// Исходный порядок вычислений служит независимым эталоном для переноса проверок и подготовки данных ребра.
fn reference_interval(
    view: &CrossingView<'_, TestGraph>,
    crossing: &Crossing,
    mode: Option<CrossingMode>,
) -> Option<Decoration> {
    let edge = view.graph.edge(crossing.edge);
    let other = view.graph.edge(crossing.other);
    let settings = view.settings(edge.id);
    let other_settings = view.settings(other.id);
    let options = settings.options.unwrap_or(view.defaults);
    let options = CrossingOptions {
        mode: mode.unwrap_or_else(|| options.effective_mode()),
        ..options
    };
    let upper = settings
        .priority
        .cmp(&other_settings.priority)
        .then_with(|| match view.defaults.order {
            CrossingOrder::StableId => std::cmp::Ordering::Equal,
            CrossingOrder::HorizontalFirst => crossing.horizontal.cmp(&crossing.other_horizontal),
            CrossingOrder::VerticalFirst => crossing.other_horizontal.cmp(&crossing.horizontal),
        })
        .then_with(|| other.id.cmp(&edge.id))
        .is_gt();
    if options.mode == CrossingMode::None
        || (options.mode == CrossingMode::Gap && upper)
        || (options.mode == CrossingMode::Bridge && !upper)
        || crossing.sine + f32::EPSILON < options.min_angle.degrees().to_radians().sin()
    {
        return None;
    }
    let width = match options.size {
        CrossingSize::Fixed(size) => size.get(),
        CrossingSize::Auto { padding } => ((view.width)(crossing.edge).max(0.0)
            + (view.width)(crossing.other).max(0.0)
            + 2.0 * padding.get())
        .min(CrossingPoints::MAX),
    };
    if width <= 0.0 {
        return None;
    }
    let half = width * 0.5 / (view.zoom * crossing.sine);
    let arrow_clearance = edge
        .direction
        .arrows()
        .iter()
        .map(|&reverse| (crossing.distance - edge.path.arrow_distance(reverse)).abs())
        .fold(f32::INFINITY, f32::min);
    let padding = options.clearance.get() / view.zoom;
    if half + padding >= crossing.clearance || arrow_clearance <= half + padding + 8.0 / view.zoom {
        return None;
    }
    Some(Decoration {
        from: crossing.distance - half,
        to: crossing.distance + half,
        bridge: options.mode == CrossingMode::Bridge && options.bridge_height.get() > 0.0,
    })
}

fn reference_crowded(
    cache: &CrossingCache,
    view: &CrossingView<'_, TestGraph>,
    crossing: &Crossing,
    target: Decoration,
) -> bool {
    cache.crossings_for(crossing.edge).iter().any(|other| {
        !std::ptr::eq(other, crossing)
            && reference_interval(view, other, None)
                .is_some_and(|next| next.from <= target.to && next.to >= target.from)
    })
}

fn reference_decoration(
    cache: &CrossingCache,
    view: &CrossingView<'_, TestGraph>,
    crossing: &Crossing,
) -> Option<Decoration> {
    if let Some(decoration) = reference_interval(view, crossing, None) {
        return (!decoration.bridge || !reference_crowded(cache, view, crossing, decoration))
            .then_some(decoration);
    }
    if view.options(crossing.edge).mode != CrossingMode::Bridge
        || view.options(crossing.other).mode != CrossingMode::Bridge
    {
        return None;
    }
    let upper = cache
        .crossings_on_segments(
            crossing.other,
            crossing.other_segment..crossing.other_segment + 1,
        )
        .iter()
        .find(|other| other.other == crossing.edge && other.fraction == crossing.other_fraction)?;
    let decoration = reference_interval(view, upper, None)?;
    if !decoration.bridge || !reference_crowded(cache, view, upper, decoration) {
        return None;
    }
    reference_interval(view, crossing, Some(CrossingMode::Gap))
}

fn bits(decoration: Decoration) -> (u32, u32, bool) {
    (
        decoration.from.to_bits(),
        decoration.to.to_bits(),
        decoration.bridge,
    )
}

#[test]
fn prepared_edge_views_preserve_decoration_intervals_and_crowded_fallbacks() {
    let (mut graph, mut cache) =
        intersecting_lines(&[25.0, 40.0, 95.0, 105.0, 160.0, 175.0], 200.0);
    let threshold = CrossingAngle::new(15.0)
        .unwrap()
        .degrees()
        .to_radians()
        .sin();
    for (index, sine) in [
        1.0,
        0.5,
        1.0,
        1.0,
        threshold - f32::EPSILON,
        threshold - 2.0 * f32::EPSILON,
    ]
    .into_iter()
    .enumerate()
    {
        cache.crossings[index].sine = sine;
        cache.crossings[index + 6].sine = sine;
    }
    let points = |value| CrossingPoints::new(value).unwrap();
    let options = [
        CrossingOptions::default(),
        CrossingOptions {
            size: CrossingSize::Fixed(points(0.0)),
            ..CrossingOptions::default()
        },
        CrossingOptions {
            size: CrossingSize::Fixed(points(128.0)),
            ..CrossingOptions::default()
        },
        CrossingOptions {
            size: CrossingSize::Auto {
                padding: points(3.0),
            },
            ..CrossingOptions::default()
        },
        CrossingOptions {
            min_angle: CrossingAngle::new(1.0).unwrap(),
            clearance: points(0.0),
            ..CrossingOptions::default()
        },
        CrossingOptions {
            min_angle: CrossingAngle::new(90.0).unwrap(),
            clearance: points(128.0),
            ..CrossingOptions::default()
        },
    ];
    let overrides = [
        BTreeMap::new(),
        BTreeMap::from([
            (
                EdgeId::new(0),
                EdgeCrossings {
                    priority: -1,
                    options: None,
                },
            ),
            (
                EdgeId::new(2),
                EdgeCrossings {
                    priority: 9,
                    options: Some(options[3]),
                },
            ),
            (
                EdgeId::new(3),
                EdgeCrossings {
                    priority: -3,
                    options: Some(CrossingOptions {
                        mode: CrossingMode::None,
                        ..options[0]
                    }),
                },
            ),
            (
                EdgeId::new(4),
                EdgeCrossings {
                    priority: -3,
                    options: Some(CrossingOptions {
                        mode: CrossingMode::Bridge,
                        ..options[4]
                    }),
                },
            ),
        ]),
    ];
    let width = |index| [2.0, -1.0, f32::NAN, 8.0, f32::INFINITY, 0.0, 4.0][index];
    for direction in [
        Direction::Undirected,
        Direction::Forward,
        Direction::Reverse,
        Direction::Both,
    ] {
        graph.direction = direction;
        for mode in [CrossingMode::None, CrossingMode::Gap, CrossingMode::Bridge] {
            for order in [
                CrossingOrder::StableId,
                CrossingOrder::HorizontalFirst,
                CrossingOrder::VerticalFirst,
            ] {
                for zoom in [0.125, 1.0, 64.0] {
                    for height in [0.0, 6.0] {
                        for options in options {
                            for overrides in &overrides {
                                let view = CrossingView {
                                    graph: &graph,
                                    defaults: CrossingOptions {
                                        mode,
                                        order,
                                        bridge_height: points(height),
                                        ..options
                                    },
                                    overrides,
                                    zoom,
                                    width: &width,
                                };
                                for index in 0..graph.edge_count() {
                                    let edge_view = view.edge(index);
                                    for crossing in cache.crossings_for(index) {
                                        assert_eq!(
                                            cache.decoration(crossing, &view, &edge_view).map(bits),
                                            reference_decoration(&cache, &view, crossing).map(bits),
                                            "crossing={crossing:?}, options={:?}, direction={direction:?}, zoom={zoom}, overrides={overrides:?}",
                                            view.defaults,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn disjoint_bridges_reuse_their_edge_and_skip_reading_unrelated_edges() {
    let positions: Vec<_> = (1..65).map(|index| index as f32 * 20.0).collect();
    let (graph, cache) = intersecting_lines(&positions, 1300.0);
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
    let expected: Vec<_> = cache
        .crossings_for(0)
        .iter()
        .filter_map(|crossing| reference_decoration(&cache, &view, crossing))
        .map(bits)
        .collect();
    let reference_reads = graph.reads.replace(0);
    let actual: Vec<_> = cache.decorations(0, &view).into_iter().map(bits).collect();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), positions.len());
    assert!(graph.reads.get() * 8 < reference_reads);
}
