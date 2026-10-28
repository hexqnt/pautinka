use egui::{Rect, Vec2, pos2};

use super::*;

fn viewport() -> Rect {
    Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(1000.0))
}

fn request(id: u64, position: LabelPosition) -> Request {
    Request {
        id: NodeId::new(id),
        index: id as usize,
        node: Rect::from_center_size(pos2(500.0, 500.0), Vec2::splat(10.0)),
        size: Vec2::new(100.0, 20.0),
        options: NodeLabelOptions {
            placement: LabelPlacement::Fixed(position),
            ..NodeLabelOptions::default()
        },
        forced: false,
        importance: 0,
    }
}

#[test]
fn stable_identity_and_priority_determine_the_winner_independent_of_input_order() {
    for reversed in [false, true] {
        let mut layout = Layout::default();
        let mut requests = [
            request(9, LabelPosition::East),
            request(1, LabelPosition::East),
        ];
        if reversed {
            requests.reverse();
        }
        layout.requests.extend(requests);
        layout.solve(viewport(), LabelLayoutOptions::default());
        assert_eq!(layout.placed.len(), 1);
        assert_eq!(layout.placed[0].id, NodeId::new(1));
        layout
            .requests
            .iter_mut()
            .find(|request| request.id == NodeId::new(9))
            .unwrap()
            .options
            .priority = 100;
        layout.solve(viewport(), LabelLayoutOptions::default());
        assert_eq!(layout.placed.len(), 1);
        assert_eq!(layout.placed[0].id, NodeId::new(9));
    }
}

#[test]
fn all_eight_positions_respect_node_bounds_and_gap() {
    let node = request(0, LabelPosition::East).node;
    for position in LabelPosition::ALL {
        let rect = position.rect(node, Vec2::new(100.0, 20.0), 4.0);
        assert!(!rect.intersects(node.expand(3.99)));
        assert_eq!(rect.size(), Vec2::new(100.0, 20.0));
    }
    let vertical = LabelPositions::new(LabelPosition::North).with(LabelPosition::South);
    assert_eq!(vertical.iter().count(), 2);
}

#[test]
fn clear_candidates_win_even_with_a_large_preference_penalty() {
    let mut layout = Layout::default();
    let mut node = request(0, LabelPosition::East);
    node.options.placement = LabelPlacement::Auto {
        allowed: LabelPositions::ALL,
        preferred: LabelPosition::East,
    };
    layout
        .nodes
        .push(LabelPosition::East.rect(node.node, node.size, 4.0));
    layout.requests.push(node);
    let options = LabelLayoutOptions {
        weights: LabelWeights {
            preference: 1000.0,
            ..LabelWeights::default()
        },
        ..LabelLayoutOptions::default()
    };
    layout.solve(viewport(), options);
    assert_eq!(layout.placed.len(), 1);
    assert_ne!(layout.placed[0].position, LabelPosition::East);
    assert!(!layout.placed[0].rect.intersects(layout.nodes[0]));
}

#[test]
fn budget_and_conflicts_can_only_be_overridden_by_forced_labels() {
    let mut layout = Layout::default();
    let normal = request(0, LabelPosition::East);
    layout.requests.push(normal);
    layout.solve(
        viewport(),
        LabelLayoutOptions {
            max_labels: 0,
            ..LabelLayoutOptions::default()
        },
    );
    assert_eq!(layout.placed, []);
    layout.requests[0].forced = true;
    layout.nodes.push(normal.node.expand(300.0));
    layout.solve(
        viewport(),
        LabelLayoutOptions {
            max_labels: 0,
            ..LabelLayoutOptions::default()
        },
    );
    assert_eq!(layout.placed.len(), 1);
    assert!(layout.placed[0].forced);
    assert!(layout.placed[0].conflicted);
}

#[test]
fn visibility_hysteresis_uses_separate_reveal_and_hide_thresholds() {
    let mut layout = Layout::default();
    layout.requests.push(request(0, LabelPosition::East));
    let options = LabelLayoutOptions {
        label_padding: 0.0,
        node_padding: 0.0,
        hide_above: 0.1,
        reveal_below: 0.02,
        weights: LabelWeights {
            nodes: 1.0,
            ..LabelWeights::default()
        },
        ..LabelLayoutOptions::default()
    };
    layout.solve(viewport(), options);
    assert_eq!(layout.placed.len(), 1);
    layout
        .nodes
        .push(Rect::from_min_max(pos2(604.0, 490.0), pos2(610.0, 510.0)));
    layout.solve(viewport(), options);
    assert_eq!(layout.placed.len(), 1);
    layout.solve(
        viewport(),
        LabelLayoutOptions {
            max_labels: 0,
            ..options
        },
    );
    assert_eq!(layout.placed, []);
    layout.solve(viewport(), options);
    assert_eq!(layout.placed, []);
    layout.nodes[0].min.x = 608.0;
    layout.solve(viewport(), options);
    assert_eq!(layout.placed.len(), 1);
}

#[test]
fn small_preference_changes_keep_the_previous_position_and_zero_margin_allows_switching() {
    let mut layout = Layout::default();
    let mut node = request(0, LabelPosition::East);
    node.options.placement = LabelPlacement::Auto {
        allowed: LabelPositions::ALL,
        preferred: LabelPosition::East,
    };
    layout.requests.push(node);
    layout.solve(viewport(), LabelLayoutOptions::default());
    assert_eq!(layout.placed[0].position, LabelPosition::East);
    layout.requests[0].options.placement = LabelPlacement::Auto {
        allowed: LabelPositions::ALL,
        preferred: LabelPosition::West,
    };
    layout.solve(viewport(), LabelLayoutOptions::default());
    assert_eq!(layout.placed[0].position, LabelPosition::East);
    layout.solve(
        viewport(),
        LabelLayoutOptions {
            switch_margin: 0.0,
            ..LabelLayoutOptions::default()
        },
    );
    assert_eq!(layout.placed[0].position, LabelPosition::West);
}

#[test]
fn stationary_frames_and_interior_pan_reuse_the_layout_but_viewport_edges_invalidate_it() {
    let mut layout = Layout::default();
    layout.requests.push(request(0, LabelPosition::East));
    layout.solve(viewport(), LabelLayoutOptions::default());
    let rect = layout.placed[0].rect;
    layout.solve(viewport(), LabelLayoutOptions::default());
    assert_eq!(layout.solves, 1);
    let delta = Vec2::new(3.0, 1.0);
    layout.requests[0].node = layout.requests[0].node.translate(delta);
    layout.solve(viewport(), LabelLayoutOptions::default());
    assert_eq!(layout.solves, 1);
    assert_eq!(layout.placed[0].rect, rect.translate(delta));
    layout.requests[0].node = layout.requests[0].node.translate(Vec2::new(450.0, 0.0));
    layout.solve(viewport(), LabelLayoutOptions::default());
    assert_eq!(layout.solves, 2);
    assert_eq!(layout.placed, []);
}

#[test]
fn multi_cell_obstacles_are_counted_once_and_zero_weight_disables_collisions() {
    let mut grid = grid::Grid::default();
    grid.clear(viewport());
    let rect = Rect::from_min_size(pos2(100.0, 100.0), Vec2::new(300.0, 200.0));
    grid.insert(rect, 1.0);
    assert_eq!(grid.overlap(rect), 1.0);
    grid.insert(rect, 2.0);
    assert_eq!(grid.overlap(rect), 3.0);
    grid.clear(viewport());
    grid.insert(rect, 0.0);
    assert_eq!(grid.overlap(rect), 0.0);
}

#[test]
fn label_clearance_considers_obstacles_just_outside_the_viewport_during_pan() {
    let mut layout = Layout::default();
    let mut label = request(0, LabelPosition::West);
    label.node = Rect::from_min_max(pos2(104.5, 495.0), pos2(114.5, 505.0));
    layout.requests.push(label);
    layout
        .nodes
        .push(Rect::from_min_max(pos2(-1.0, 490.0), pos2(-0.5, 510.0)));
    let options = LabelLayoutOptions {
        label_padding: 4.0,
        node_padding: 0.0,
        ..LabelLayoutOptions::default()
    };
    layout.solve(viewport(), options);
    assert_eq!(layout.placed, []);
    let delta = Vec2::new(2.0, 0.0);
    layout.requests[0].node = layout.requests[0].node.translate(delta);
    layout.nodes[0] = layout.nodes[0].translate(delta);
    layout.solve(viewport(), options);
    assert_eq!(layout.placed, []);
    assert_eq!(layout.solves, 1);
    layout.nodes.clear();
    layout.solve(viewport(), options);
    assert_eq!(layout.placed.len(), 1);
}

#[test]
fn extreme_finite_rectangles_keep_a_finite_area() {
    let rect = Rect::from_min_max(pos2(-f32::MAX, -f32::MAX), pos2(f32::MAX, f32::MAX));
    let area = grid::area(rect);
    assert!(area.is_finite());
    assert!(area > f64::from(f32::MAX));
}

#[test]
fn restricted_positions_and_disabled_collision_weights_are_respected() {
    let mut layout = Layout::default();
    let mut label = request(0, LabelPosition::East);
    label.options.placement = LabelPlacement::Auto {
        allowed: LabelPositions::new(LabelPosition::North).with(LabelPosition::South),
        preferred: LabelPosition::North,
    };
    layout
        .nodes
        .push(LabelPosition::North.rect(label.node, label.size, 4.0));
    layout.requests.push(label);
    layout.solve(viewport(), LabelLayoutOptions::default());
    assert_eq!(layout.placed[0].position, LabelPosition::South);
    layout.solve(
        viewport(),
        LabelLayoutOptions {
            weights: LabelWeights {
                labels: 0.0,
                nodes: 0.0,
                viewport: 0.0,
                preference: 1.0,
            },
            switch_margin: 0.0,
            ..LabelLayoutOptions::default()
        },
    );
    assert_eq!(layout.placed[0].position, LabelPosition::North);
}

#[test]
fn invalid_settings_are_rejected_and_empty_viewports_do_not_panic() {
    assert!(LabelLayoutOptions::default().valid());
    for value in [f32::NAN, f32::INFINITY, -1.0] {
        assert!(
            !LabelLayoutOptions {
                gap: value,
                ..LabelLayoutOptions::default()
            }
            .valid()
        );
    }
    assert!(
        !LabelLayoutOptions {
            reveal_below: 1.0,
            hide_above: 0.0,
            ..LabelLayoutOptions::default()
        }
        .valid()
    );
    let mut layout = Layout::default();
    layout.requests.push(request(0, LabelPosition::East));
    layout.solve(Rect::ZERO, LabelLayoutOptions::default());
    assert_eq!(layout.placed, []);
}
