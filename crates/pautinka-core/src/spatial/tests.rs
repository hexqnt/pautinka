use std::collections::HashSet;

use emath::{Pos2, Vec2, pos2};

use super::*;

mod bulk;

fn rect(x: f32, y: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), Vec2::splat(4.0))
}

#[test]
fn queries_match_linear_scans_before_and_after_local_updates() {
    let mut entries = (0..1000)
        .map(|i| (i, rect((i % 32) as f32 * 10.0, (i / 32) as f32 * 10.0)))
        .collect::<Vec<_>>();
    let mut index = SpatialIndex::default();
    index.rebuild(entries.iter().copied()).unwrap();
    for round in 0..3 {
        for x in -1..12 {
            for y in -1..12 {
                let query = Rect::from_min_size(
                    pos2(x as f32 * 28.0, y as f32 * 28.0),
                    Vec2::new(17.0, 23.0),
                );
                let mut actual = Vec::new();
                index.query(query, |key| actual.push(key));
                actual.sort_unstable();
                let expected = entries
                    .iter()
                    .filter(|(_, bounds)| bounds.intersects(query))
                    .map(|&(key, _)| key)
                    .collect::<Vec<_>>();
                assert_eq!(actual, expected);
            }
        }
        for (ordinal, (_, bounds)) in entries.iter_mut().enumerate().step_by(7) {
            *bounds = rect((ordinal % 13) as f32 * 17.0 - 20.0, round as f32 * 30.0);
            index
                .update(index.handle(ordinal).unwrap(), *bounds)
                .unwrap();
        }
    }
}

fn assert_pairs(index: &SpatialIndex<usize>, entries: impl IntoIterator<Item = (usize, Rect)>) {
    let entries = entries.into_iter().collect::<Vec<_>>();
    let ordered = |a: usize, b: usize| (a.min(b), a.max(b));
    let mut actual = Vec::new();
    index.query_pairs(|a, b| actual.push(ordered(a, b)));
    actual.sort_unstable();
    let mut expected = Vec::new();
    for (position, &(a, bounds)) in entries.iter().enumerate() {
        for &(b, other) in &entries[position + 1..] {
            if bounds.intersects(other) {
                expected.push(ordered(a, b));
            }
        }
    }
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

#[test]
fn pair_queries_match_linear_scan_after_moves_removals_and_insertions() {
    let mut random = 43_u64;
    let mut coordinate = || {
        random = random
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        ((random >> 32) % 256) as f32 - 128.0
    };
    let mut next_bounds = || {
        let min = pos2(coordinate(), coordinate());
        Rect::from_min_size(
            min,
            Vec2::new(coordinate().abs() % 40.0, coordinate().abs() % 40.0),
        )
    };
    let mut index = SpatialIndex::default();
    let initial = (0..256).map(|key| (key, next_bounds())).collect::<Vec<_>>();
    index.rebuild(initial.iter().copied()).unwrap();
    let mut entries = initial
        .into_iter()
        .map(|(key, bounds)| Some((index.handle(key).unwrap(), key, bounds)))
        .collect::<Vec<_>>();
    for round in 0..4 {
        assert_pairs(
            &index,
            entries
                .iter()
                .flatten()
                .map(|&(_, key, bounds)| (key, bounds)),
        );
        assert_tree(&index);
        for (handle, _, bounds) in entries.iter_mut().step_by(7).flatten() {
            *bounds = next_bounds();
            index.update(*handle, *bounds).unwrap();
        }
        if round == 0 {
            for entry in entries.iter_mut().step_by(11) {
                if let Some((handle, _, _)) = entry.take() {
                    index.remove(handle).unwrap();
                }
            }
        } else if round == 1 {
            for key in 256..300 {
                let bounds = next_bounds();
                entries.push(Some((index.insert(key, bounds).unwrap(), key, bounds)));
            }
        }
    }
}

#[test]
fn pair_queries_include_touches_and_duplicate_values_without_self_pairs() {
    let mut index = SpatialIndex::default();
    assert_pairs(&index, []);
    index.rebuild([(7, Rect::ZERO)]).unwrap();
    assert_pairs(&index, [(7, Rect::ZERO)]);
    let identical = [(7, Rect::ZERO); 32];
    index.rebuild(identical).unwrap();
    assert_pairs(&index, identical);
    let entries = [
        (0, Rect::ZERO),
        (0, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0))),
        (1, Rect::from_min_max(pos2(1.0, 1.0), pos2(1.0, 1.0))),
        (1, Rect::from_min_max(pos2(1.0, -1.0), pos2(1.0, 2.0))),
        (2, Rect::from_min_max(pos2(2.0, 0.0), pos2(2.0, 0.0))),
    ];
    index.rebuild(entries).unwrap();
    assert_pairs(&index, entries);
}

#[test]
fn handles_reject_other_owners_and_successful_rebuilds_but_survive_rejected_input() {
    let mut first = SpatialIndex::default();
    let mut second = SpatialIndex::default();
    first.rebuild([(0, rect(0.0, 0.0))]).unwrap();
    second.rebuild([(0, rect(1.0, 1.0))]).unwrap();
    let handle = first.handle(0).unwrap();
    assert_eq!(
        second.update(handle, rect(2.0, 2.0)),
        Err(SpatialError::InvalidHandle)
    );
    assert_eq!(
        first.rebuild([(0, Rect::NOTHING)]),
        Err(SpatialError::InvalidBounds)
    );
    assert!(first.update(handle, rect(3.0, 3.0)).is_ok());
    first.rebuild([(0, rect(0.0, 0.0))]).unwrap();
    assert_eq!(first.bounds(handle), Err(SpatialError::InvalidHandle));
    assert_eq!(
        first.update(
            first.handle(0).unwrap(),
            Rect::from_min_max(pos2(1.0, 1.0), Pos2::ZERO)
        ),
        Err(SpatialError::InvalidBounds)
    );
}

#[test]
fn empty_identical_and_zero_area_bounds_are_supported() {
    let mut index = SpatialIndex::default();
    index.rebuild([]).unwrap();
    assert!(index.is_empty());
    index.query(Rect::ZERO, |_: usize| {
        panic!("Empty index returned a value")
    });
    index.rebuild([(7, Rect::ZERO); 100]).unwrap();
    let mut count = 0;
    index.query(Rect::ZERO, |_| count += 1);
    assert_eq!(count, 100);
    assert!(size_of::<TreeNode>() <= 40);
}

fn assert_tree(index: &SpatialIndex<usize>) {
    fn visit(index: &SpatialIndex<usize>, node: TreeIndex, visited: &mut HashSet<usize>) -> usize {
        assert!(visited.insert(node.offset()), "Spatial tree has a cycle");
        let current = &index.nodes[node.offset()];
        match current.children {
            Children::Leaf(_) => {
                assert_eq!(current.height, 0);
                1
            }
            Children::Branch(left, right) => {
                assert_eq!(
                    index.nodes[left.offset()].parent.map(TreeIndex::offset),
                    Some(node.offset())
                );
                assert_eq!(
                    index.nodes[right.offset()].parent.map(TreeIndex::offset),
                    Some(node.offset())
                );
                let a = &index.nodes[left.offset()];
                let b = &index.nodes[right.offset()];
                assert!(
                    a.height.abs_diff(b.height) <= 1,
                    "Spatial tree is unbalanced"
                );
                assert_eq!(current.height, 1 + a.height.max(b.height));
                assert_eq!(current.bounds, a.bounds.union(b.bounds));
                visit(index, left, visited) + visit(index, right, visited)
            }
            Children::Pending => panic!("Freed spatial node is reachable"),
        }
    }
    let count = index.root.map_or(0, |root| {
        assert!(index.nodes[root.offset()].parent.is_none());
        visit(index, root, &mut HashSet::new())
    });
    assert_eq!(count, index.len());
}

#[test]
fn rotations_preserve_both_orientations_and_equal_height_tie_breaks() {
    fn subtree(index: &mut SpatialIndex<usize>, height: u16) -> TreeIndex {
        if height == 0 {
            let key = index.values.len();
            index.values.push(key);
            index.count += 1;
            return index.allocate_node(rect(key as f32, 0.0), Children::Leaf(key as u32));
        }
        let left = subtree(index, height - 1);
        let right = subtree(index, height - 1);
        let parent = index.allocate_node(Rect::NOTHING, Children::Pending);
        index.set_branch(parent, left, right);
        parent
    }

    for right_heavy in [false, true] {
        for (a_height, b_height) in [(1, 0), (0, 1), (1, 1)] {
            let mut index = SpatialIndex::default();
            let a = subtree(&mut index, a_height);
            let b = subtree(&mut index, b_height);
            let tall = index.allocate_node(Rect::NOTHING, Children::Pending);
            index.set_branch(tall, a, b);
            let short = subtree(&mut index, 0);
            let root = index.allocate_node(Rect::NOTHING, Children::Pending);
            let (left, right) = if right_heavy {
                (short, tall)
            } else {
                (tall, short)
            };
            index.set_branch(root, left, right);
            index.root = Some(root);
            let (upper, lower) = if a_height > b_height { (a, b) } else { (b, a) };

            assert_eq!(index.balance(root).offset(), tall.offset());
            let expected = if right_heavy {
                (short, lower, root, upper)
            } else {
                (lower, short, upper, root)
            };
            let Children::Branch(left, right) = index.nodes[root.offset()].children else {
                panic!("Rotated node must be a branch")
            };
            assert_eq!(
                (left.offset(), right.offset()),
                (expected.0.offset(), expected.1.offset())
            );
            let Children::Branch(left, right) = index.nodes[tall.offset()].children else {
                panic!("Promoted node must be a branch")
            };
            assert_eq!(
                (left.offset(), right.offset()),
                (expected.2.offset(), expected.3.offset())
            );
            assert_tree(&index);
        }
    }
}

#[test]
fn mixed_insert_move_remove_preserves_queries_balance_and_slot_generations() {
    let mut index = SpatialIndex::default();
    let mut entries: Vec<Option<(SpatialHandle, Rect)>> = Vec::new();
    for key in 0..128 {
        let bounds = rect(key as f32 * 7.0, (key % 11) as f32 * 13.0);
        entries.push(Some((index.insert(key, bounds).unwrap(), bounds)));
        assert_tree(&index);
    }
    let mut retired = Vec::new();
    let mut random = 17u64;
    for step in 0..2000 {
        random = random
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let bounds = rect(
            ((random >> 32) % 1000) as f32,
            ((random >> 16) % 1000) as f32,
        );
        if step % 3 == 0 || index.is_empty() {
            let key = entries.len();
            let handle = index.insert(key, bounds).unwrap();
            entries.push(Some((handle, bounds)));
        } else {
            let active = random as usize % index.len();
            let (_, entry) = entries
                .iter_mut()
                .enumerate()
                .filter(|(_, entry)| entry.is_some())
                .nth(active)
                .unwrap();
            let (handle, old) = entry.as_mut().unwrap();
            if step % 3 == 1 {
                index.update(*handle, bounds).unwrap();
                *old = bounds;
            } else {
                let handle = *handle;
                index.remove(handle).unwrap();
                retired.push(handle);
                *entry = None;
                assert_eq!(index.bounds(handle), Err(SpatialError::InvalidHandle));
            }
        }
        assert_tree(&index);
        for &handle in retired.iter().rev().take(8) {
            assert_eq!(index.bounds(handle), Err(SpatialError::InvalidHandle));
        }
        if step % 20 == 0 {
            let query = bounds.expand(200.0);
            let mut found = Vec::new();
            index.query(query, |key| found.push(key));
            found.sort_unstable();
            let expected = entries
                .iter()
                .enumerate()
                .filter_map(|(key, entry)| {
                    entry
                        .filter(|(_, rect)| rect.intersects(query))
                        .map(|_| key)
                })
                .collect::<Vec<_>>();
            assert_eq!(found, expected);
        }
    }
    for entry in entries.into_iter().flatten() {
        index.remove(entry.0).unwrap();
    }
    assert!(index.is_empty());
    assert_tree(&index);
}

#[test]
fn contained_moves_and_distant_insertions_keep_ancestor_bounds_exact() {
    let mut index = SpatialIndex::default();
    let outer = Rect::from_min_max(pos2(-100.0, -100.0), pos2(100.0, 100.0));
    index.rebuild([(0, outer), (1, Rect::ZERO)]).unwrap();
    let inner = index.handle(1).unwrap();
    let moved = rect(50.0, 60.0);
    index.update(inner, moved).unwrap();
    assert_eq!(index.total_bounds(), Some(outer));
    assert_tree(&index);
    let mut found = Vec::new();
    index.query(Rect::ZERO, |key| found.push(key));
    assert_eq!(found, [0]);
    for key in 2..20 {
        let bounds = rect(key as f32 * 200.0, key as f32 * -150.0);
        index.insert(key, bounds).unwrap();
        assert_tree(&index);
        assert!(index.total_bounds().unwrap().contains_rect(bounds));
        let mut found = Vec::new();
        index.query(bounds, |key| found.push(key));
        assert_eq!(found, [key]);
    }
    index.update(inner, Rect::ZERO).unwrap();
    assert_tree(&index);
    let mut found = Vec::new();
    index.query(Rect::ZERO, |key| found.push(key));
    found.sort_unstable();
    assert_eq!(found, [0, 1]);
}
