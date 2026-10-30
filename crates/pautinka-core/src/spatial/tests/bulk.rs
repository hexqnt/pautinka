use super::*;

#[test]
fn batched_queries_and_pairs_match_linear_scans_after_mixed_edits() {
    for mode in [SpatialUpdateMode::Incremental, SpatialUpdateMode::Refit] {
        let mut index = SpatialIndex::default();
        let mut entries: Vec<_> = (0..256)
            .map(|key| {
                (
                    key,
                    rect((key % 16) as f32 * 10.0, (key / 16) as f32 * 10.0),
                )
            })
            .collect();
        index.rebuild(entries.iter().copied()).unwrap();
        let mut handles: Vec<_> = (0..entries.len())
            .map(|key| index.handle(key).unwrap())
            .collect();
        // Предварительные повороты исключают зависимость обхода от порядка узлов в буфере.
        for &handle in handles.iter().step_by(3) {
            index.update(handle, rect(-100.0, 300.0)).unwrap();
        }
        for round in 0..4 {
            {
                let mut updates = index.updates(mode);
                for ((key, bounds), &handle) in entries.iter_mut().zip(&handles) {
                    *bounds = rect(
                        ((*key * 37 + round * 13) % 128) as f32,
                        ((*key * 19 + round * 7) % 128) as f32,
                    );
                    updates.update(handle, *bounds).unwrap();
                }
                for position in (0..entries.len()).step_by(11).rev() {
                    let (key, _) = entries.remove(position);
                    let retired = handles.remove(position);
                    assert_eq!(updates.remove(retired), Ok(key));
                    assert_eq!(updates.remove(retired), Err(SpatialError::InvalidHandle));
                }
                for key in 256 + round * 20..276 + round * 20 {
                    let bounds = rect((key % 13) as f32 * 10.0, (key % 17) as f32 * 10.0);
                    handles.push(updates.insert(key, bounds).unwrap());
                    entries.push((key, bounds));
                }
            }
            assert_tree(&index);
            assert_pairs(&index, entries.iter().copied());
            for (&handle, &(_, bounds)) in handles.iter().zip(&entries) {
                assert_eq!(index.bounds(handle), Ok(bounds));
            }
            for x in -1..12 {
                let query = rect(x as f32 * 15.0, 50.0).expand(20.0);
                let mut actual = Vec::new();
                index.query(query, |key| actual.push(key));
                actual.sort_unstable();
                let mut expected: Vec<_> = entries
                    .iter()
                    .filter(|(_, bounds)| bounds.intersects(query))
                    .map(|&(key, _)| key)
                    .collect();
                expected.sort_unstable();
                assert_eq!(actual, expected);
            }
        }
    }
}

#[test]
fn coherent_bulk_motion_preserves_partition_handles_and_storage() {
    let mut index = SpatialIndex::default();
    index
        .rebuild((0..1024).map(|key| {
            (
                key,
                rect((key % 32) as f32 * 10.0, (key / 32) as f32 * 10.0),
            )
        }))
        .unwrap();
    let handles: Vec<_> = (0..index.len())
        .map(|key| index.handle(key).unwrap())
        .collect();
    let leaves = index.leaves.clone();
    let bytes = index.storage_bytes();
    let quality = index.refit_quality;
    for round in 0..40 {
        {
            let mut updates = index.updates(SpatialUpdateMode::Refit);
            for (key, &handle) in handles.iter().enumerate() {
                let offset = round as f32 * 13.0;
                updates
                    .update(
                        handle,
                        rect(
                            (key % 32) as f32 * 10.0 + offset,
                            (key / 32) as f32 * 10.0 - offset,
                        ),
                    )
                    .unwrap();
            }
        }
        assert_tree(&index);
        assert!(index.leaves == leaves);
        assert_eq!(index.refit_quality, quality);
        assert_eq!(index.storage_bytes(), bytes);
    }
}

#[test]
fn spatial_scrambling_repartitions_without_invalidating_handles_or_allocating() {
    let mut index = SpatialIndex::default();
    let count = 1024_u32;
    index
        .rebuild((0..count).map(|key| (key as usize, rect(key as f32 * 10.0, 0.0))))
        .unwrap();
    let handles: Vec<_> = (0..index.len())
        .map(|key| index.handle(key).unwrap())
        .collect();
    let leaves = index.leaves.clone();
    let bytes = index.storage_bytes();
    let bounds: Vec<_> = (0..count)
        .map(|key| rect((key.reverse_bits() >> (u32::BITS - 10)) as f32 * 10.0, 0.0))
        .collect();
    {
        let mut updates = index.updates(SpatialUpdateMode::Refit);
        for (&handle, &bounds) in handles.iter().zip(&bounds) {
            updates.update(handle, bounds).unwrap();
        }
    }
    assert_tree(&index);
    assert!(index.leaves != leaves);
    assert!(index.quality() <= index.refit_quality * 2.0);
    assert_eq!(index.storage_bytes(), bytes);
    for (&handle, &bounds) in handles.iter().zip(&bounds) {
        assert_eq!(index.bounds(handle), Ok(bounds));
    }
}

#[test]
fn early_errors_and_unwinding_leave_applied_updates_queryable() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let mut index = SpatialIndex::default();
    index
        .rebuild([(0, rect(0.0, 0.0)), (1, rect(10.0, 0.0))])
        .unwrap();
    let handle = index.handle(0).unwrap();
    let mut foreign = SpatialIndex::default();
    let foreign_handle = foreign.insert(2, rect(0.0, 0.0)).unwrap();
    let moved = rect(100.0, 100.0);
    let result = (|| {
        let mut updates = index.updates(SpatialUpdateMode::Refit);
        updates.update(handle, moved)?;
        assert_eq!(
            updates.update(handle, Rect::NOTHING),
            Err(SpatialError::InvalidBounds)
        );
        updates.update(foreign_handle, Rect::ZERO)
    })();
    assert_eq!(result, Err(SpatialError::InvalidHandle));
    assert_tree(&index);
    assert_eq!(index.bounds(handle), Ok(moved));
    let moved = rect(-100.0, -100.0);
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let mut updates = index.updates(SpatialUpdateMode::Refit);
            updates.update(handle, moved).unwrap();
            panic!("Abort batch");
        }))
        .is_err()
    );
    assert_tree(&index);
    let mut found = Vec::new();
    index.query(moved, |key| found.push(key));
    assert_eq!(found, [0]);
}

#[test]
fn empty_single_and_insert_only_trees_support_bulk_updates() {
    for count in [0, 1, 16] {
        let mut index = SpatialIndex::default();
        for key in 0..count {
            index.insert(key, Rect::ZERO).unwrap();
        }
        {
            let mut updates = index.updates(SpatialUpdateMode::Refit);
            for key in 0..count {
                updates
                    .update(updates.handle(key).unwrap(), rect(key as f32 * 10.0, 0.0))
                    .unwrap();
            }
        }
        assert_tree(&index);
        {
            let mut updates = index.updates(SpatialUpdateMode::Refit);
            for key in 0..count {
                updates.remove(updates.handle(key).unwrap()).unwrap();
            }
        }
        assert!(index.is_empty());
        assert_tree(&index);
    }
}
