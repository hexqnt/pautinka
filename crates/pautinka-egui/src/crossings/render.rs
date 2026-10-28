use std::collections::BTreeMap;

use egui::{Pos2, Vec2};

use crate::{Edge, EdgeId, Graph, geometry::Curve};

use super::{
    BridgeSide, Crossing, CrossingCache, CrossingMode, CrossingOptions, CrossingOrder,
    CrossingPoints, CrossingSize, EdgeCrossings,
};

#[cfg(test)]
#[path = "tests/render.rs"]
mod tests;

#[derive(Clone, Copy)]
struct Decoration {
    from: f32,
    to: f32,
    bridge: bool,
}

// Данные ребра и настройки общие для всех его пересечений, включая проверку тесных мостиков.
struct EdgeCrossingView<'a> {
    edge: Edge<'a>,
    options: CrossingOptions,
    priority: i32,
    minimum_sine: f32,
    padding: f32,
}

pub(crate) struct CrossingView<'a, G: ?Sized> {
    pub graph: &'a G,
    pub defaults: CrossingOptions,
    pub overrides: &'a BTreeMap<EdgeId, EdgeCrossings>,
    pub zoom: f32,
    pub width: &'a dyn Fn(usize) -> f32,
}

impl<G: Graph + ?Sized> CrossingView<'_, G> {
    fn settings(&self, id: EdgeId) -> EdgeCrossings {
        self.overrides.get(&id).copied().unwrap_or_default()
    }

    pub fn options(&self, index: usize) -> CrossingOptions {
        // Без переопределений ID не нужен; этот путь выполняется для каждого видимого ребра.
        let options = if self.overrides.is_empty() {
            self.defaults
        } else {
            self.settings(self.graph.edge(index).id)
                .options
                .unwrap_or(self.defaults)
        };
        CrossingOptions {
            mode: options.effective_mode(),
            ..options
        }
    }

    fn edge(&self, index: usize) -> EdgeCrossingView<'_> {
        let edge = self.graph.edge(index);
        let settings = self.settings(edge.id);
        let options = settings.options.unwrap_or(self.defaults);
        let options = CrossingOptions {
            mode: options.effective_mode(),
            ..options
        };
        EdgeCrossingView {
            edge,
            options,
            priority: settings.priority,
            minimum_sine: options.min_angle.degrees().to_radians().sin(),
            padding: options.clearance.get() / self.zoom,
        }
    }
}

impl EdgeCrossingView<'_> {
    fn interval<G: Graph + ?Sized>(
        &self,
        crossing: &Crossing,
        view: &CrossingView<'_, G>,
        mode: Option<CrossingMode>,
    ) -> Option<Decoration> {
        let options = CrossingOptions {
            mode: mode.unwrap_or(self.options.mode),
            ..self.options
        };
        if options.mode == CrossingMode::None || crossing.sine + f32::EPSILON < self.minimum_sine {
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
        if half + self.padding >= crossing.clearance {
            return None;
        }
        let arrow_clearance = self
            .edge
            .direction
            .arrows()
            .iter()
            .map(|&reverse| (crossing.distance - self.edge.path.arrow_distance(reverse)).abs())
            .fold(f32::INFINITY, f32::min);
        if arrow_clearance <= half + self.padding + 8.0 / view.zoom {
            return None;
        }
        Some(Decoration {
            from: crossing.distance - half,
            to: crossing.distance + half,
            bridge: options.mode == CrossingMode::Bridge && options.bridge_height.get() > 0.0,
        })
    }

    fn applies<G: Graph + ?Sized>(
        &self,
        crossing: &Crossing,
        view: &CrossingView<'_, G>,
        mode: Option<CrossingMode>,
    ) -> bool {
        let other = view.graph.edge(crossing.other);
        let other_settings = view.settings(other.id);
        let upper = self
            .priority
            .cmp(&other_settings.priority)
            .then_with(|| match view.defaults.order {
                CrossingOrder::StableId => std::cmp::Ordering::Equal,
                CrossingOrder::HorizontalFirst => {
                    crossing.horizontal.cmp(&crossing.other_horizontal)
                }
                CrossingOrder::VerticalFirst => crossing.other_horizontal.cmp(&crossing.horizontal),
            })
            .then_with(|| other.id.cmp(&self.edge.id))
            .is_gt();
        match mode.unwrap_or(self.options.mode) {
            CrossingMode::None => false,
            CrossingMode::Gap => !upper,
            CrossingMode::Bridge => upper,
        }
    }

    fn decoration<G: Graph + ?Sized>(
        &self,
        crossing: &Crossing,
        view: &CrossingView<'_, G>,
        mode: Option<CrossingMode>,
    ) -> Option<Decoration> {
        self.interval(crossing, view, mode)
            .filter(|_| self.applies(crossing, view, mode))
    }
}

impl CrossingCache {
    fn crossings_for(&self, index: usize) -> &[Crossing] {
        self.offsets
            .get(index..)
            .and_then(|offsets| offsets.first_chunk::<2>())
            .map_or(&[], |&[start, end]| &self.crossings[start..end])
    }

    fn crossings_on_segments(&self, index: usize, segments: std::ops::Range<usize>) -> &[Crossing] {
        let crossings = self.crossings_for(index);
        let start = crossings.partition_point(|crossing| crossing.segment < segments.start);
        let end = crossings[start..].partition_point(|crossing| crossing.segment < segments.end);
        &crossings[start..start + end]
    }

    /// Дуги целого ребра либо одной части пространственного индекса выбора.
    pub fn bridges<G: Graph + ?Sized>(
        &self,
        index: usize,
        chunk: Option<usize>,
        view: &CrossingView<'_, G>,
        mut emit: impl FnMut(&[Pos2]),
    ) {
        if self.crossings_for(index).is_empty() {
            return;
        }
        let options = view.options(index);
        if options.mode != CrossingMode::Bridge || !self.ready(view.graph) {
            return;
        }
        let edge_view = view.edge(index);
        let path = edge_view.edge.path;
        let crossings = if let Some(chunk) = chunk {
            let Some(segments) = path.spatial_chunk_segments(chunk) else {
                return;
            };
            self.crossings_on_segments(index, segments)
        } else {
            self.crossings_for(index)
        };
        for crossing in crossings {
            let Some(Decoration {
                from,
                to,
                bridge: true,
            }) = self.decoration(crossing, view, &edge_view)
            else {
                continue;
            };
            let a = path.at_distance(from).expect("Crossing distance is finite");
            let b = path.at_distance(to).expect("Crossing distance is finite");
            let height = options.bridge_height.get() / view.zoom;
            let points = if matches!(path.curve(), Curve::Polyline(_)) {
                bridge_points(a.position, b.position, height, options.bridge_side)
            } else {
                let handle = a.position.distance(b.position) * 0.25;
                bridge_with_handles(
                    a.position,
                    b.position,
                    [a.tangent * handle, b.tangent * handle],
                    height,
                    options.bridge_side,
                )
            };
            emit(&points);
        }
    }

    fn decorations<G: Graph + ?Sized>(
        &self,
        index: usize,
        view: &CrossingView<'_, G>,
    ) -> smallvec::SmallVec<[Decoration; 8]> {
        let crossings = self.crossings_for(index);
        if crossings.is_empty()
            || view.options(index).mode == CrossingMode::None
            || !self.ready(view.graph)
        {
            return smallvec::SmallVec::new();
        }
        let edge_view = view.edge(index);
        let mut decorations: smallvec::SmallVec<[_; 8]> = crossings
            .iter()
            .filter_map(|crossing| self.decoration(crossing, view, &edge_view))
            .collect();
        // Поправка на угол может менять порядок начал относительно центров пересечений.
        decorations.sort_unstable_by(|a, b| a.from.total_cmp(&b.from));
        decorations
    }

    /// Части исходного пути; кривые сохраняют контрольные точки вместо превращения в ломаную.
    pub fn curves<G: Graph + ?Sized>(
        &self,
        index: usize,
        view: &CrossingView<'_, G>,
        mut emit: impl FnMut(Curve<'_>),
    ) {
        let path = view.graph.edge(index).path;
        if matches!(path.curve(), Curve::Polyline(_)) {
            self.pieces(index, view, |points| emit(Curve::Polyline(points)));
            return;
        }
        let decorations = self.decorations(index, view);
        if decorations.is_empty() {
            emit(path.curve());
            return;
        }
        let mut from = 0.0_f32;
        for decoration in decorations {
            if decoration.from > from {
                emit(super::curves::section_at_distance(
                    path,
                    from,
                    decoration.from,
                ));
            }
            from = from.max(decoration.to);
        }
        if from < path.length() {
            emit(super::curves::section_at_distance(
                path,
                from,
                path.length(),
            ));
        }
    }

    fn crowded<G: Graph + ?Sized>(
        &self,
        crossing: &Crossing,
        target: Decoration,
        view: &CrossingView<'_, G>,
        edge_view: &EdgeCrossingView<'_>,
    ) -> bool {
        self.crossings_for(crossing.edge).iter().any(|other| {
            // Второе ребро и приоритет нужны только при совпадении геометрических интервалов.
            !std::ptr::eq(other, crossing)
                && edge_view.interval(other, view, None).is_some_and(|next| {
                    next.from <= target.to
                        && next.to >= target.from
                        && edge_view.applies(other, view, None)
                })
        })
    }

    fn decoration<G: Graph + ?Sized>(
        &self,
        crossing: &Crossing,
        view: &CrossingView<'_, G>,
        edge_view: &EdgeCrossingView<'_>,
    ) -> Option<Decoration> {
        if let Some(decoration) = edge_view.decoration(crossing, view, None) {
            return (!decoration.bridge || !self.crowded(crossing, decoration, view, edge_view))
                .then_some(decoration);
        }
        if edge_view.options.mode != CrossingMode::Bridge
            || view.options(crossing.other).mode != CrossingMode::Bridge
        {
            return None;
        }
        let upper = self
            .crossings_on_segments(
                crossing.other,
                crossing.other_segment..crossing.other_segment + 1,
            )
            .iter()
            .find(|other| {
                other.other == crossing.edge && other.fraction == crossing.other_fraction
            })?;
        let upper_view = view.edge(crossing.other);
        let decoration = upper_view.decoration(upper, view, None)?;
        if !decoration.bridge || !self.crowded(upper, decoration, view, &upper_view) {
            return None;
        }
        edge_view.decoration(crossing, view, Some(CrossingMode::Gap))
    }

    /// Части исходной линии без разрывов и мостиков; рабочий буфер живёт только во время вызова `emit`.
    pub fn pieces<G: Graph + ?Sized>(
        &self,
        index: usize,
        view: &CrossingView<'_, G>,
        mut emit: impl FnMut(&[Pos2]),
    ) {
        let edge = view.graph.edge(index);
        let points = edge.path.points();
        let decorations = self.decorations(index, view);
        if decorations.is_empty() {
            emit(points);
            return;
        }
        let mut decorations = decorations.into_iter().peekable();
        // Короткие части маршрута помещаются в стековый буфер.
        let mut piece = smallvec::SmallVec::<[Pos2; 8]>::new();
        piece.push(points[0]);
        let mut distance = 0.0;
        let mut gap_end = 0.0_f32;
        for pair in points.windows(2) {
            let [a, b] = [pair[0], pair[1]];
            let length = a.distance(b);
            if length == 0.0 {
                continue;
            }
            let end = distance + length;
            while let Some(Decoration { from, to, .. }) =
                decorations.next_if(|decoration| decoration.from <= end)
            {
                if from > gap_end {
                    if piece.is_empty() {
                        piece.push(a.lerp(b, ((gap_end - distance) / length).max(0.0)));
                    }
                    piece.push(a.lerp(b, (from - distance) / length));
                    emit(&piece);
                    piece.clear();
                }
                gap_end = gap_end.max(to);
            }
            if gap_end < end {
                if piece.is_empty() {
                    piece.push(a.lerp(b, ((gap_end - distance) / length).max(0.0)));
                }
                piece.push(b);
            }
            distance = end;
        }
        if piece.len() >= 2 {
            emit(&piece);
        }
    }
}

/// Две кубические кривые; дискретизация общая для рисования и выбора.
pub(super) fn bridge_points(a: Pos2, b: Pos2, height: f32, side: BridgeSide) -> [Pos2; 25] {
    bridge_with_handles(a, b, [(b - a) * 0.25; 2], height, side)
}

fn bridge_with_handles(
    a: Pos2,
    b: Pos2,
    handles: [Vec2; 2],
    height: f32,
    side: BridgeSide,
) -> [Pos2; 25] {
    // Локальная интерполяция позволяет векторизовать отсчёты, сохраняя порядок округлений egui::lerp.
    #[inline]
    fn lerp(a: Pos2, b: Pos2, t: f32) -> Pos2 {
        Pos2 {
            x: egui::lerp(a.x..=b.x, t),
            y: egui::lerp(a.y..=b.y, t),
        }
    }

    let delta = b - a;
    let mut normal = delta.normalized().rot90();
    match side {
        BridgeSide::Left => {}
        BridgeSide::Right => normal = -normal,
        BridgeSide::Auto => {
            if (delta.x.abs() >= delta.y.abs() && normal.y > 0.0)
                || (delta.x.abs() < delta.y.abs() && normal.x < 0.0)
            {
                normal = -normal;
            }
        }
    }
    let top = lerp(a, b, 0.5) + normal * height;
    let handle = delta * 0.25;
    let curves = [
        [a, a + handles[0], top - handle, top],
        [top, top + handle, b - handles[1], b],
    ];
    let mut points = [Pos2::ZERO; 25];
    let (first_half, second_half) = points.split_at_mut(13);
    // Вершина входит только в первую половину; внутри цикла кривая уже выбрана.
    for (half, curve, offset) in [(first_half, curves[0], 0), (second_half, curves[1], 1)] {
        for (index, point) in half.iter_mut().enumerate() {
            let t = (index + offset) as f32 / 12.0;
            let [a, b, c, d] = curve;
            let middle = lerp(b, c, t);
            let first = lerp(lerp(a, b, t), middle, t);
            let second = lerp(middle, lerp(c, d, t), t);
            *point = lerp(first, second, t);
        }
    }
    points
}
