use crate::{Pos2, Vec2};

/// Точка выбора хранится в f64: добавление смещения указателя к большой мировой позиции не теряет экранную точность.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitPoint {
    x: f64,
    y: f64,
}

impl HitPoint {
    pub fn new(x: f64, y: f64) -> Option<Self> {
        let limit = f64::from(f32::MAX);
        (x.is_finite() && y.is_finite() && x.abs() <= limit && y.abs() <= limit)
            .then_some(Self { x, y })
    }
    pub fn from_world(point: Pos2) -> Option<Self> {
        Self::new(f64::from(point.x), f64::from(point.y))
    }
    pub fn coordinates(self) -> [f64; 2] {
        [self.x, self.y]
    }
    pub fn rounded(self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }
    pub fn relative_to(self, origin: Pos2) -> Vec2 {
        Vec2::new(
            (self.x - f64::from(origin.x)) as f32,
            (self.y - f64::from(origin.y)) as f32,
        )
    }
    /// Квадрат расстояния до отрезка с конечными концами; вычисления сохраняют точность локального смещения и вырожденных отрезков.
    #[inline]
    pub fn segment_distance_sq(self, a: Pos2, b: Pos2) -> f64 {
        let delta = [
            f64::from(b.x) - f64::from(a.x),
            f64::from(b.y) - f64::from(a.y),
        ];
        let relative = [self.x - f64::from(a.x), self.y - f64::from(a.y)];
        let length = delta[0] * delta[0] + delta[1] * delta[1];
        let t = if length > 0.0 {
            ((relative[0] * delta[0] + relative[1] * delta[1]) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        (relative[0] - delta[0] * t).powi(2) + (relative[1] - delta[1] * t).powi(2)
    }
    /// Запас для консервативного поиска в индексе, хранящем границы в f32.
    pub fn rounding_error(self) -> f32 {
        let rounded = self.rounded();
        (self.x - f64::from(rounded.x))
            .abs()
            .max((self.y - f64::from(rounded.y)).abs()) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_distances_cover_endpoints_degeneracy_precision_and_extreme_units() {
        let a = Pos2::ZERO;
        let b = Pos2::new(10.0, 0.0);
        for (x, expected) in [(-1.0, 10.0), (5.0, 9.0), (11.0, 10.0)] {
            let point = HitPoint::new(x, 3.0).unwrap();
            assert_eq!(point.segment_distance_sq(a, b), expected);
            assert_eq!(point.segment_distance_sq(b, a), expected);
        }
        let point = HitPoint::new(5.0, 3.0).unwrap();
        assert_eq!(point.segment_distance_sq(a, a), 34.0);
        let origin = Pos2::new(1e9, -1e9);
        let point = HitPoint::new(1e9 + 0.25, -1e9 - 0.5).unwrap();
        assert_eq!(
            point.segment_distance_sq(origin, origin + Vec2::X * 1024.0),
            0.25
        );
        let tiny = f32::from_bits(1);
        let point = HitPoint::new(f64::from(tiny) * 0.5, f64::from(tiny) * 2.0).unwrap();
        assert_eq!(
            point.segment_distance_sq(a, Pos2::new(tiny, 0.0)),
            (f64::from(tiny) * 2.0).powi(2)
        );
        let point = HitPoint::new(0.0, f64::from(f32::MAX)).unwrap();
        assert_eq!(
            point.segment_distance_sq(Pos2::new(-f32::MAX, 0.0), Pos2::new(f32::MAX, 0.0)),
            f64::from(f32::MAX).powi(2)
        );
        let point = HitPoint::new(-0.0, -0.0).unwrap();
        assert_eq!(
            point
                .segment_distance_sq(a, Pos2::new(-0.0, -0.0))
                .to_bits(),
            0.0_f64.to_bits()
        );
    }

    #[test]
    fn preserves_local_offsets_and_bounds_rounding_error() {
        let origin = Pos2::new(1e9, 1e9);
        let point = HitPoint::new(1e9 + 0.25, 1e9 - 0.5).unwrap();
        assert_eq!(point.relative_to(origin), Vec2::new(0.25, -0.5));
        assert_eq!(point.rounded(), origin);
        assert_eq!(point.rounding_error(), 0.5);
        assert!(HitPoint::new(f64::NAN, 0.0).is_none());
        assert!(HitPoint::new(f64::INFINITY, 0.0).is_none());
        assert!(HitPoint::new(f64::from(f32::MAX) * 2.0, 0.0).is_none());
    }
}
