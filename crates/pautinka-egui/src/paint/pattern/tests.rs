use super::*;
use egui::{Color32, pos2};

fn marks(pattern: LinePattern, points: &[Pos2]) -> Vec<Shape> {
    let pattern = Pattern::new(pattern, 2.0);
    let runs: &[_] = match &pattern.0 {
        Kind::Dotted(runs) => runs,
        Kind::DashDot(runs) => runs,
        Kind::DashDotDot(runs) => runs,
        _ => panic!("Expected a dotted pattern"),
    };
    let mut shapes = Vec::new();
    paint_marks(points, runs, Stroke::new(2.0, Color32::WHITE), |shape| {
        shapes.push(shape);
    });
    shapes
}

#[test]
fn dots_have_round_shapes_and_gaps_between_their_edges() {
    let shapes = marks(
        LinePattern::Dotted { gap: 3.0 },
        &[Pos2::ZERO, pos2(13.0, 0.0)],
    );
    let dots: Vec<_> = shapes
        .iter()
        .map(|shape| {
            let Shape::Circle(circle) = shape else {
                panic!("Expected a circle")
            };
            assert_eq!(circle.radius, 1.0);
            circle.center
        })
        .collect();
    assert_eq!(dots, [pos2(1.0, 0.0), pos2(6.0, 0.0), pos2(11.0, 0.0)]);
}

#[test]
fn mixed_patterns_repeat_dashes_and_the_requested_number_of_dots() {
    for (pattern, centers, second_dash) in [
        (
            LinePattern::DashDot {
                dash: 4.0,
                gap: 3.0,
            },
            [8.0, 20.0],
            12.0,
        ),
        (
            LinePattern::DashDotDot {
                dash: 4.0,
                gap: 3.0,
            },
            [8.0, 13.0],
            17.0,
        ),
    ] {
        let shapes = marks(pattern, &[Pos2::ZERO, pos2(23.0, 0.0)]);
        let dots: Vec<_> = shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Circle(circle) => Some(circle.center.x),
                _ => None,
            })
            .collect();
        assert_eq!(dots, centers);
        let dashes: Vec<_> = shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::LineSegment { points, .. } => Some(*points),
                _ => None,
            })
            .collect();
        assert_eq!(
            dashes,
            [
                [Pos2::ZERO, pos2(4.0, 0.0)],
                [pos2(second_dash, 0.0), pos2(second_dash + 4.0, 0.0)]
            ]
        );
    }
}

#[test]
fn marks_continue_across_bends_and_repeated_vertices() {
    let shapes = marks(
        LinePattern::DashDot {
            dash: 4.0,
            gap: 3.0,
        },
        &[
            Pos2::ZERO,
            Pos2::ZERO,
            pos2(2.0, 0.0),
            pos2(2.0, 2.0),
            pos2(2.0, 6.0),
            pos2(2.0, 6.0),
            pos2(2.0, 15.0),
        ],
    );
    let expected = [
        Shape::line_segment(
            [Pos2::ZERO, pos2(2.0, 0.0)],
            Stroke::new(2.0, Color32::WHITE),
        ),
        Shape::line_segment(
            [pos2(2.0, 0.0), pos2(2.0, 2.0)],
            Stroke::new(2.0, Color32::WHITE),
        ),
        Shape::circle_filled(pos2(2.0, 6.0), 1.0, Color32::WHITE),
        Shape::line_segment(
            [pos2(2.0, 10.0), pos2(2.0, 14.0)],
            Stroke::new(2.0, Color32::WHITE),
        ),
    ];
    assert_eq!(shapes, expected);
}

#[test]
fn empty_and_short_paths_do_not_create_duplicate_dots() {
    let pattern = LinePattern::Dotted { gap: 3.0 };
    assert_eq!(marks(pattern, &[]), []);
    assert_eq!(marks(pattern, &[Pos2::ZERO]), []);
    assert_eq!(marks(pattern, &[Pos2::ZERO; 3]), []);
    assert_eq!(marks(pattern, &[Pos2::ZERO, pos2(0.5, 0.0)]), []);
    assert_eq!(
        marks(
            pattern,
            &[Pos2::ZERO, pos2(1.0, 0.0), pos2(1.0, 0.0), pos2(2.0, 0.0)]
        )
        .len(),
        1
    );
}

#[test]
fn subdividing_segments_preserves_dot_positions_and_dash_coverage() {
    let points: Vec<_> = (0..=100).map(|x| pos2(x as f32 * 0.5, 0.0)).collect();
    for pattern in [
        LinePattern::Dotted { gap: 3.0 },
        LinePattern::DashDot {
            dash: 4.0,
            gap: 3.0,
        },
        LinePattern::DashDotDot {
            dash: 4.0,
            gap: 3.0,
        },
    ] {
        let whole = marks(pattern, &[Pos2::ZERO, pos2(50.0, 0.0)]);
        let split = marks(pattern, &points);
        let dots = |shapes: &[Shape]| -> Vec<_> {
            shapes
                .iter()
                .filter_map(|shape| match shape {
                    Shape::Circle(circle) => Some(circle.center),
                    _ => None,
                })
                .collect()
        };
        let whole_dots = dots(&whole);
        let split_dots = dots(&split);
        assert_eq!(whole_dots.len(), split_dots.len());
        for (whole, split) in whole_dots.iter().zip(&split_dots) {
            assert!(
                whole.distance(*split) < 1e-5,
                "Dot positions differ: {whole:?}, {split:?}"
            );
        }
        let dash_length = |shapes: &[Shape]| -> f32 {
            shapes
                .iter()
                .filter_map(|shape| match shape {
                    Shape::LineSegment { points: [a, b], .. } => Some(a.distance(*b)),
                    _ => None,
                })
                .sum()
        };
        assert!((dash_length(&whole) - dash_length(&split)).abs() < 1e-5);
    }
}

#[test]
fn invalid_lengths_fall_back_to_solid_once_at_the_pattern_boundary() {
    for invalid in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for pattern in [
            LinePattern::Dotted { gap: invalid },
            LinePattern::Dashed {
                dash: invalid,
                gap: 3.0,
            },
            LinePattern::Dashed {
                dash: 4.0,
                gap: invalid,
            },
            LinePattern::DashDot {
                dash: invalid,
                gap: 3.0,
            },
            LinePattern::DashDot {
                dash: 4.0,
                gap: invalid,
            },
            LinePattern::DashDotDot {
                dash: invalid,
                gap: 3.0,
            },
            LinePattern::DashDotDot {
                dash: 4.0,
                gap: invalid,
            },
        ] {
            assert!(Pattern::new(pattern, 2.0).is_solid());
        }
        for pattern in [
            LinePattern::Dotted { gap: 3.0 },
            LinePattern::DashDot {
                dash: 4.0,
                gap: 3.0,
            },
            LinePattern::DashDotDot {
                dash: 4.0,
                gap: 3.0,
            },
        ] {
            assert!(Pattern::new(pattern, invalid).is_solid());
        }
    }
}
