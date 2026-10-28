mod index;

use emath::Pos2;

pub use index::AlignmentIndex;

/// Привязка к ближайшим горизонтали и вертикали в пределах конечного положительного допуска.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AlignmentSnap {
    tolerance: f32,
    release_tolerance: f32,
}

impl AlignmentSnap {
    pub fn new(tolerance: f32) -> Option<Self> {
        (tolerance.is_finite() && tolerance > 0.0).then_some(Self {
            tolerance,
            release_tolerance: (f64::from(tolerance) * 1.5).min(f64::from(f32::MAX)) as f32,
        })
    }

    pub fn tolerance(self) -> f32 {
        self.tolerance
    }

    /// Порог отпускания не меньше порога захвата; по умолчанию он в 1,5 раза больше.
    pub fn with_release_tolerance(self, release_tolerance: f32) -> Option<Self> {
        (release_tolerance.is_finite() && release_tolerance >= self.tolerance).then_some(Self {
            release_tolerance,
            ..self
        })
    }

    pub fn release_tolerance(self) -> f32 {
        self.release_tolerance
    }

    /// Переводит оба допуска в другую систему единиц с сохранением положительности.
    pub fn scaled(self, factor: f64) -> Option<Self> {
        if !factor.is_finite() || factor <= 0.0 {
            return None;
        }
        let scale = |value| {
            (f64::from(value) * factor).clamp(f64::from(f32::from_bits(1)), f64::from(f32::MAX))
                as f32
        };
        Some(Self {
            tolerance: scale(self.tolerance),
            release_tolerance: scale(self.release_tolerance),
        })
    }

    /// Поиск без удержания: один проход без аллокаций. Для гистерезиса используйте [`AlignmentIndex::snap`].
    /// Вызывающий исключает перемещаемые вершины из кандидатов.
    /// При равном отклонении оси выбирается ближайшая точка, затем порядок координат.
    /// Неконечные кандидаты пропускаются; неконечная позиция возвращается без привязки.
    pub fn snap(self, position: Pos2, candidates: impl IntoIterator<Item = Pos2>) -> Alignment {
        let mut result = Alignment {
            position,
            horizontal: None,
            vertical: None,
        };
        if !position.is_finite() {
            return result;
        }
        let mut best = [[f64::INFINITY; 4]; 2];
        for candidate in candidates.into_iter().filter(|p| p.is_finite()) {
            let dx = f64::from(candidate.x) - f64::from(position.x);
            let dy = f64::from(candidate.y) - f64::from(position.y);
            for ((error, other_error, source), best) in [
                (dy.abs(), dx.abs(), &mut result.horizontal),
                (dx.abs(), dy.abs(), &mut result.vertical),
            ]
            .into_iter()
            .zip(&mut best)
            {
                if error > f64::from(self.tolerance) {
                    continue;
                }
                let rank = [error, other_error, candidate.x.into(), candidate.y.into()];
                if rank < *best {
                    *best = rank;
                    *source = Some(candidate);
                }
            }
        }
        if let Some(source) = result.horizontal {
            result.position.y = source.y;
        }
        if let Some(source) = result.vertical {
            result.position.x = source.x;
        }
        result
    }
}

impl Default for AlignmentSnap {
    fn default() -> Self {
        Self {
            tolerance: 6.0,
            release_tolerance: 9.0,
        }
    }
}

/// Источники направляющих могут быть разными; их координаты заданы до привязки.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Alignment {
    pub position: Pos2,
    pub horizontal: Option<Pos2>,
    pub vertical: Option<Pos2>,
}

#[cfg(test)]
mod tests;
