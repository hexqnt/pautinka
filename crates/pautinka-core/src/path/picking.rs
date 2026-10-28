use crate::{HitPoint, Pos2, Rect};

use super::{EdgePath, PathData, SAMPLES, bezier};

impl EdgePath {
    fn chunk_stride(&self) -> usize {
        if matches!(self.data, PathData::Line(_) | PathData::Polyline { .. }) {
            1
        } else if self.bounds.width().max(self.bounds.height()) <= 256.0 {
            SAMPLES - 1
        } else {
            6
        }
    }

    fn chunk_ranges(&self) -> impl ExactSizeIterator<Item = (usize, usize)> {
        let stride = self.chunk_stride();
        let last = self.points().len() - 1;
        (0..last.div_ceil(stride)).map(move |chunk| {
            let from = chunk * stride;
            (from, (from + stride).min(last))
        })
    }

    /// Индексы сегментов в `points()`, покрываемых частью пространственного индекса.
    pub fn spatial_chunk_segments(&self, chunk: usize) -> Option<std::ops::Range<usize>> {
        self.chunk_ranges().nth(chunk).map(|(from, to)| from..to)
    }

    /// Консервативные границы частей кривой либо точные границы сегментов ломаной.
    /// Отклонение кривой от хорды ограничено максимумом второй производной.
    pub fn spatial_chunks(&self) -> impl ExactSizeIterator<Item = Rect> + '_ {
        let points = self.points();
        self.chunk_ranges().map(move |(from, to)| {
            Rect::from_points(&points[from..=to])
                .expand(
                    self.deviation_per_step * ((to - from) * (to - from)) as f32
                        + self.rounding_margin,
                )
                .intersect(self.bounds)
        })
    }

    /// Проверяет только выбранную часть пути, найденную пространственным индексом.
    pub fn chunk_distance_sq(&self, chunk: usize, point: Pos2) -> Option<f32> {
        let (from, to) = self.chunk_ranges().nth(chunk)?;
        Some(self.range_distance_sq(HitPoint::from_world(point)?, from, to) as f32)
    }

    fn range_distance_sq(&self, point: HitPoint, from: usize, to: usize) -> f64 {
        self.data.bezier_controls().map_or_else(
            || polyline_distance_sq(&self.points()[from..=to], point),
            |controls| {
                bezier::distance_sq(
                    controls,
                    point,
                    from as f32 / (SAMPLES - 1) as f32,
                    to as f32 / (SAMPLES - 1) as f32,
                )
            },
        )
    }

    /// Отбрасывает далёкие части по консервативной погрешности отсчётов; возвращает уточнённое расстояние только внутри допуска.
    pub fn chunk_hit_distance_sq(&self, chunk: usize, point: Pos2, radius: f32) -> Option<f64> {
        self.chunk_hit_distance_sq_precise(chunk, HitPoint::from_world(point)?, radius)
    }

    /// Проверка части пути без округления координат указателя до f32; допуск задаётся в мировых единицах.
    pub fn chunk_hit_distance_sq_precise(
        &self,
        chunk: usize,
        point: HitPoint,
        radius: f32,
    ) -> Option<f64> {
        if !radius.is_finite() || radius < 0.0 {
            return None;
        }
        let (from, to) = self.chunk_ranges().nth(chunk)?;
        self.range_hit_distance_sq(point, f64::from(radius), from, to)
    }

    fn range_hit_distance_sq(
        &self,
        point: HitPoint,
        radius: f64,
        from: usize,
        to: usize,
    ) -> Option<f64> {
        let coarse = polyline_distance_sq(&self.points()[from..=to], point);
        if matches!(self.data, PathData::Line(_) | PathData::Polyline { .. }) {
            return (coarse <= radius * radius).then_some(coarse);
        }
        if coarse.sqrt() > radius + f64::from(self.deviation_per_step + self.rounding_margin) {
            return None;
        }
        let distance = self.range_distance_sq(point, from, to);
        (distance <= radius * radius).then_some(distance)
    }

    /// Допуск задаётся в мировых единицах; квадрат расстояния хранится в f64, чтобы смена единиц не вызывала underflow/overflow.
    pub fn hit_distance_sq(&self, point: Pos2, radius: f32) -> Option<f64> {
        self.hit_distance_sq_precise(HitPoint::from_world(point)?, radius)
    }

    /// Проверка всего пути с сохранением точности указателя относительно больших мировых позиций.
    pub fn hit_distance_sq_precise(&self, point: HitPoint, radius: f32) -> Option<f64> {
        if !radius.is_finite() || radius < 0.0 {
            return None;
        }
        let radius = f64::from(radius);
        self.chunk_ranges()
            .filter_map(|(from, to)| self.range_hit_distance_sq(point, radius, from, to))
            .min_by(f64::total_cmp)
    }

    /// Квадрат расстояния до пути; для Bézier применяется адаптивное уточнение без аллокаций.
    pub fn distance_sq(&self, point: Pos2) -> f32 {
        let Some(point) = HitPoint::from_world(point) else {
            return f32::INFINITY;
        };
        self.range_distance_sq(point, 0, self.points().len() - 1) as f32
    }
}

fn polyline_distance_sq(points: &[Pos2], point: HitPoint) -> f64 {
    if let [a, b] = points {
        return point.segment_distance_sq(*a, *b);
    }
    points
        .windows(2)
        .map(|pair| point.segment_distance_sq(pair[0], pair[1]))
        .fold(f64::INFINITY, f64::min)
}
