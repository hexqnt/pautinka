use emath::Pos2;

use crate::{SnapGuide, SnapIndex, SnapKind, SnapKinds, SnapOptions, SnapResult, SnapTarget};

use super::{Alignment, AlignmentSnap};

/// Индекс горизонталей и вертикалей; совместимый адаптер над общим индексом привязок.
#[derive(Default)]
pub struct AlignmentIndex {
    index: SnapIndex,
}

impl AlignmentIndex {
    pub fn rebuild(&mut self, candidates: impl IntoIterator<Item = Pos2>) {
        self.index.rebuild(candidates, SnapKinds::AXES);
    }

    pub fn snap(
        &self,
        position: Pos2,
        policy: AlignmentSnap,
        previous: Option<Alignment>,
    ) -> Alignment {
        let previous = previous.map(|alignment| {
            let horizontal = alignment.horizontal.map(|source| SnapGuide {
                kind: SnapKind::Horizontal,
                source,
            });
            let vertical = alignment.vertical.map(|source| SnapGuide {
                kind: SnapKind::Vertical,
                source,
            });
            let target = match (horizontal, vertical) {
                (Some(a), Some(b)) => Some(SnapTarget::Intersection([a, b])),
                (Some(guide), None) | (None, Some(guide)) => Some(SnapTarget::Guide(guide)),
                (None, None) => None,
            };
            SnapResult {
                position: alignment.position,
                target,
            }
        });
        let result = self.index.snap(
            position,
            SnapOptions {
                kinds: SnapKinds::AXES,
                tolerance: policy,
            },
            previous,
        );
        Alignment {
            position: result.position,
            horizontal: result
                .guides()
                .find(|guide| guide.kind == SnapKind::Horizontal)
                .map(|guide| guide.source),
            vertical: result
                .guides()
                .find(|guide| guide.kind == SnapKind::Vertical)
                .map(|guide| guide.source),
        }
    }
}
