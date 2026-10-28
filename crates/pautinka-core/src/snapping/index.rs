use std::cmp::Ordering;

use emath::Pos2;

use super::choice::Choice;
use super::{SnapGrid, SnapGuide, SnapKind, SnapKinds, SnapOptions, SnapResult, SnapTarget};

/// Отсортированные индексы семейств направляющих: O(n log n) на подготовку, O(log n) на запрос.
/// Запросы не выделяют память; буферы сохраняются при смене режимов и перестройке.
#[derive(Default)]
pub struct SnapIndex {
    families: [Vec<Pos2>; 4],
}

impl SnapIndex {
    /// При изменении набора режимов индекс нужно подготовить заново. Неконечные точки отбрасываются.
    pub fn rebuild(&mut self, candidates: impl IntoIterator<Item = Pos2>, kinds: SnapKinds) {
        for points in &mut self.families {
            points.clear();
        }
        let Some(first) = SnapKind::ALL
            .into_iter()
            .find(|kind| kinds.contains(kind.flag()))
        else {
            return;
        };
        let [base, others @ ..] = &mut self.families[first.index()..] else {
            unreachable!("An active snap family must exist");
        };
        base.extend(
            candidates
                .into_iter()
                .filter(|point| point.is_finite())
                .map(|mut point| {
                    // Обе записи нуля образуют одну координату, включая сортировку вдоль направляющей.
                    if point.x == 0.0 {
                        point.x = 0.0;
                    }
                    if point.y == 0.0 {
                        point.y = 0.0;
                    }
                    point
                }),
        );
        let diagonal =
            kinds.contains(SnapKinds::DIAGONAL_UP) || kinds.contains(SnapKinds::DIAGONAL_DOWN);
        if diagonal {
            // При сильно разных порядках координат диагональный компаратор теряет младшую координату даже в f64. Исходный порядок x/y сохраняет выбор источника и группирует точные дубликаты.
            base.sort_unstable_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
        } else {
            sort(base, first);
        }
        base.dedup();
        for (&kind, points) in SnapKind::ALL[first.index() + 1..].iter().zip(others) {
            if kinds.contains(kind.flag()) {
                points.extend_from_slice(base);
                sort(points, kind);
            }
        }
        if diagonal {
            sort(base, first);
        }
    }

    /// Удержание имеет приоритет над новым захватом, затем выбираются пересечения и ближайшая цель.
    /// Слишком далёкие пересечения отбрасываются: перемещение ограничено диагональю квадрата допуска.
    pub fn snap(
        &self,
        position: Pos2,
        options: SnapOptions,
        previous: Option<SnapResult>,
    ) -> SnapResult {
        self.snap_with_grid(position, options, None, previous)
    }

    /// Сравнивает сетку с результатом штатного поиска направляющих без последовательных смещений.
    /// Удерживаемая цель имеет приоритет; среди новых целей выбирается ближайшая, при равенстве — направляющая.
    pub fn snap_with_grid(
        &self,
        position: Pos2,
        options: SnapOptions,
        grid: Option<SnapGrid>,
        previous: Option<SnapResult>,
    ) -> SnapResult {
        let unsnapped = SnapResult {
            position,
            target: None,
        };
        if !position.is_finite() {
            return unsnapped;
        }
        if options.kinds.is_empty() {
            return grid.map_or(unsnapped, |grid| {
                grid.snap(position, options.tolerance, previous)
            });
        }
        let candidates = SnapKind::ALL.map(|kind| {
            if self.families[kind.index()].is_empty() || !options.kinds.contains(kind.flag()) {
                return None;
            }
            let previous =
                previous.and_then(|result| result.guides().find(|guide| guide.kind == kind));
            self.source(position, kind, options, previous)
        });
        let mut best: Option<Choice> = None;
        for (i, candidate) in candidates.iter().enumerate() {
            let Some(a) = candidate else { continue };
            if let Some(choice) = Choice::project(position, *a) {
                choose(&mut best, choice);
            }
            for b in candidates[i + 1..].iter().flatten() {
                if let Some(choice) = Choice::intersect(position, *a, *b, options) {
                    choose(&mut best, choice);
                }
            }
        }
        if let Some(candidate) =
            grid.and_then(|grid| grid.candidate(position, options.tolerance, previous))
            && best
                .as_ref()
                .is_none_or(|guide| candidate.preferred_over(guide))
        {
            return candidate.result;
        }
        best.map_or(unsnapped, |choice| choice.result)
    }

    fn source(
        &self,
        position: Pos2,
        kind: SnapKind,
        options: SnapOptions,
        previous: Option<SnapGuide>,
    ) -> Option<Candidate> {
        if let Some(previous) = previous.filter(|guide| guide.source.is_finite())
            && kind.distance(position, previous.source)
                <= f64::from(options.tolerance.release_tolerance())
            && let Some(source) =
                self.nearest_on_line(position, kind, kind.coordinate(previous.source))
        {
            return Some(Candidate {
                guide: SnapGuide { kind, source },
                held: true,
            });
        }
        let points = &self.families[kind.index()];
        let coordinate = kind.coordinate(position);
        let split = points.partition_point(|point| kind.coordinate(*point) < coordinate);
        points[split.saturating_sub(1)..(split + 1).min(points.len())]
            .iter()
            .filter(|source| {
                kind.distance(position, **source) <= f64::from(options.tolerance.tolerance())
            })
            .filter_map(|source| self.nearest_on_line(position, kind, kind.coordinate(*source)))
            .min_by(|a, b| compare(position, *a, *b, kind))
            .map(|source| Candidate {
                guide: SnapGuide { kind, source },
                held: false,
            })
    }

    fn nearest_on_line(&self, position: Pos2, kind: SnapKind, coordinate: f64) -> Option<Pos2> {
        let points = &self.families[kind.index()];
        let start = points.partition_point(|point| kind.coordinate(*point) < coordinate);
        let end = points.partition_point(|point| kind.coordinate(*point) <= coordinate);
        let group = &points[start..end];
        let split = group.partition_point(|point| kind.along(*point) < kind.along(position));
        group[split.saturating_sub(1)..(split + 1).min(group.len())]
            .iter()
            .copied()
            .min_by(|a, b| compare(position, *a, *b, kind))
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    guide: SnapGuide,
    held: bool,
}

impl Choice {
    fn project(position: Pos2, candidate: Candidate) -> Option<Self> {
        let SnapGuide { kind, source } = candidate.guide;
        let [a, b] = kind.normal();
        let correction = (kind.coordinate(source) - kind.coordinate(position)) / (a * a + b * b);
        Self::new(
            position,
            [
                f64::from(position.x) + a * correction,
                f64::from(position.y) + b * correction,
            ],
            SnapTarget::Guide(candidate.guide),
            u8::from(candidate.held),
            1,
        )
    }

    fn intersect(
        position: Pos2,
        first: Candidate,
        second: Candidate,
        options: SnapOptions,
    ) -> Option<Self> {
        let [a, b] = first.guide.kind.normal();
        let [c, d] = second.guide.kind.normal();
        let p = first.guide.kind.coordinate(first.guide.source);
        let q = second.guide.kind.coordinate(second.guide.source);
        let determinant = a * d - b * c;
        let held = u8::from(first.held) + u8::from(second.held);
        let choice = Self::new(
            position,
            [(p * d - b * q) / determinant, (a * q - p * c) / determinant],
            SnapTarget::Intersection([first.guide, second.guide]),
            held,
            2,
        )?;
        let tolerance = f64::from(if held > 0 {
            options.tolerance.release_tolerance()
        } else {
            options.tolerance.tolerance()
        });
        (choice.distance_squared <= 2.0 * tolerance * tolerance).then_some(choice)
    }
}

fn choose(best: &mut Option<Choice>, candidate: Choice) {
    if best.as_ref().is_none_or(|best| {
        candidate
            .held
            .cmp(&best.held)
            .then_with(|| candidate.count.cmp(&best.count))
            .then_with(|| best.distance_squared.total_cmp(&candidate.distance_squared))
            .is_gt()
    }) {
        *best = Some(candidate);
    }
}

fn sort(points: &mut [Pos2], kind: SnapKind) {
    points.sort_unstable_by(|a, b| {
        kind.coordinate(*a)
            .total_cmp(&kind.coordinate(*b))
            .then_with(|| kind.along(*a).total_cmp(&kind.along(*b)))
    });
}

fn compare(position: Pos2, a: Pos2, b: Pos2, kind: SnapKind) -> Ordering {
    kind.distance(position, a)
        .total_cmp(&kind.distance(position, b))
        .then_with(|| {
            (kind.along(a) - kind.along(position))
                .abs()
                .total_cmp(&(kind.along(b) - kind.along(position)).abs())
        })
        .then_with(|| a.x.total_cmp(&b.x))
        .then_with(|| a.y.total_cmp(&b.y))
}
