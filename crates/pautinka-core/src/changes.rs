use std::collections::VecDeque;

use crate::{ElementIndex, GraphRevision};

const CAPACITY: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeCoverage {
    Complete,
    Unavailable,
}

#[derive(Clone, Copy)]
struct Change {
    geometry: u64,
    element: ElementIndex,
}

/// Ограниченный журнал для локальной инвалидации. После заполнения не выделяет память в куче.
pub struct GeometryChanges {
    latest: GraphRevision,
    known_from: u64,
    entries: VecDeque<Change>,
    capacity: usize,
}

impl GeometryChanges {
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            latest: GraphRevision::default(),
            known_from: 0,
            entries: VecDeque::with_capacity(capacity),
            capacity,
        }
    }
    pub fn record(
        &mut self,
        before: GraphRevision,
        after: GraphRevision,
        elements: impl IntoIterator<Item = ElementIndex>,
    ) {
        if !self.latest.same_geometry(before) {
            self.entries.clear();
            self.known_from = before.geometry();
        }
        if before.topology() != after.topology() || after.geometry() <= before.geometry() {
            self.entries.clear();
            self.known_from = after.geometry();
            self.latest = after;
            return;
        }
        for element in elements {
            if self.entries.len() == self.capacity
                && let Some(discarded) = self.entries.pop_front()
            {
                self.known_from = self.known_from.max(discarded.geometry);
            }
            self.entries.push_back(Change {
                geometry: after.geometry(),
                element,
            });
        }
        self.latest = after;
    }

    pub fn visit(
        &self,
        since: GraphRevision,
        current: GraphRevision,
        visitor: &mut dyn FnMut(ElementIndex),
    ) -> ChangeCoverage {
        if since.same_geometry(current) {
            return ChangeCoverage::Complete;
        }
        if !current.same_geometry(self.latest)
            || since.topology() != current.topology()
            || since.geometry() < self.known_from
            || since.geometry() > current.geometry()
        {
            return ChangeCoverage::Unavailable;
        }
        for change in &self.entries {
            if change.geometry > since.geometry() {
                visitor(change.element);
            }
        }
        ChangeCoverage::Complete
    }
}

impl Default for GeometryChanges {
    fn default() -> Self {
        Self::with_capacity(CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_covers_all_changes_or_explicitly_requests_a_rebuild() {
        let mut changes = GeometryChanges::default();
        let first = GraphRevision::default();
        let second = first.next_geometry();
        changes.record(
            first,
            second,
            [ElementIndex::Node(1), ElementIndex::Edge(2)],
        );
        let mut seen = Vec::new();
        assert_eq!(
            changes.visit(first, second, &mut |element| seen.push(element)),
            ChangeCoverage::Complete
        );
        assert_eq!(seen, [ElementIndex::Node(1), ElementIndex::Edge(2)]);
        let third = second.next_geometry();
        changes.record(second, third, (0..CAPACITY).map(ElementIndex::Node));
        assert_eq!(
            changes.visit(first, third, &mut |_| {}),
            ChangeCoverage::Unavailable
        );
        let mut count = 0;
        assert_eq!(
            changes.visit(second, third, &mut |_| count += 1),
            ChangeCoverage::Complete
        );
        assert_eq!(count, CAPACITY);
        assert_eq!(
            changes.visit(third, third.next_topology(), &mut |_| {}),
            ChangeCoverage::Unavailable
        );
    }
    #[test]
    fn gaps_and_topology_changes_cannot_be_reported_as_complete() {
        let mut changes = GeometryChanges::default();
        let first = GraphRevision::default();
        let skipped = first.next_geometry();
        let current = skipped.next_geometry();
        changes.record(skipped, current, [ElementIndex::Node(0)]);
        assert_eq!(
            changes.visit(first, current, &mut |_| {}),
            ChangeCoverage::Unavailable
        );
        let next = current.next_topology();
        changes.record(current, next, [ElementIndex::Node(1)]);
        assert_eq!(
            changes.visit(current, next, &mut |_| {}),
            ChangeCoverage::Unavailable
        );
    }
}
