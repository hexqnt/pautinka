use emath::Pos2;

use super::{EdgePath, PathData, PathError};

const MAX_ARC_STEP: f64 = std::f64::consts::PI / 36.0;

/// Конечный неотрицательный радиус в координатах схемы; ноль отключает скругление.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadius(f32);

impl CornerRadius {
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl TryFrom<f32> for CornerRadius {
    type Error = PathError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(PathError::InvalidCornerRadius)
        }
    }
}

impl EdgePath {
    /// Скругляет углы ломаной касательными круговыми дугами; прямые и Безье сохраняет.
    ///
    /// Дуги записываются в общую геометрию отрезками с шагом не более 5° (ошибка хорды менее 0,001 радиуса).
    /// Отступ от угла ограничен половиной каждого соседнего сегмента, чтобы дуги не перекрывались.
    /// Преобразование выполняется при построении пути, а не при отрисовке; итоговый буфер принадлежит пути.
    /// Исходные точки сохраняются для `unrounded_curve`; для изменения радиуса следует заново построить исходную ломаную.
    pub fn with_corner_radius(mut self, radius: CornerRadius) -> Result<Self, PathError> {
        let PathData::Polyline {
            points, unrounded, ..
        } = &mut self.data
        else {
            return Ok(self);
        };
        if radius.get() == 0.0 {
            return Ok(self);
        }
        let mut distinct = points.chunk_by(|a, b| a == b).map(|group| group[0]);
        let Some(mut previous) = distinct.next() else {
            return Ok(self);
        };
        let Some(mut vertex) = distinct.next() else {
            return Ok(self);
        };
        let mut rounded = Vec::with_capacity(points.len());
        rounded.push(previous);
        let mut corners = Vec::new();
        for next in distinct {
            if !append_corner(&mut rounded, previous, vertex, next, radius) {
                corners.push(rounded.len() - 1);
            }
            previous = vertex;
            vertex = next;
        }
        push_distinct(&mut rounded, vertex);
        let unrounded = unrounded.take().unwrap_or_else(|| std::mem::take(points));
        Self::build(PathData::Polyline {
            points: rounded.into_boxed_slice(),
            unrounded: Some(unrounded),
            // Обычно углов не остаётся и пустой Box не выделяет память; развороты сохраняются явно.
            corners: Some(corners.into_boxed_slice()),
        })
    }
}

fn append_corner(
    out: &mut Vec<Pos2>,
    previous: Pos2,
    vertex: Pos2,
    next: Pos2,
    radius: CornerRadius,
) -> bool {
    // Локальная геометрия в f64 избегает переполнения разностей конечных f32-координат.
    let incoming = [
        f64::from(vertex.x) - f64::from(previous.x),
        f64::from(vertex.y) - f64::from(previous.y),
    ];
    let outgoing = [
        f64::from(next.x) - f64::from(vertex.x),
        f64::from(next.y) - f64::from(vertex.y),
    ];
    let before = incoming[0].hypot(incoming[1]);
    let after = outgoing[0].hypot(outgoing[1]);
    let u = incoming.map(|value| value / before);
    let v = outgoing.map(|value| value / after);
    let cross = u[0] * v[1] - u[1] * v[0];
    let dot = u[0] * v[0] + u[1] * v[1];
    if cross == 0.0 {
        push_distinct(out, vertex);
        return dot >= 0.0;
    }
    let angle = cross.atan2(dot);
    // Эквивалент tan(|angle| / 2), устойчивый около нуля и разворота на 180°.
    let tangent = if dot >= 0.0 {
        cross.abs() / (1.0 + dot)
    } else {
        (1.0 - dot) / cross.abs()
    };
    let trim = (f64::from(radius.get()) * tangent)
        .min(before * 0.5)
        .min(after * 0.5);
    let effective_radius = trim / tangent;
    let point = |x: f64, y: f64| {
        Pos2::new(
            (f64::from(vertex.x) + x) as f32,
            (f64::from(vertex.y) + y) as f32,
        )
    };
    let start = [-u[0] * trim, -u[1] * trim];
    let radial = [
        u[1] * angle.signum() * effective_radius,
        -u[0] * angle.signum() * effective_radius,
    ];
    push_distinct(out, point(start[0], start[1]));
    // Модуль ненулевого угла не больше π: число шагов лежит в 1..=36.
    #[allow(clippy::cast_sign_loss)]
    let steps = (angle.abs() / MAX_ARC_STEP).ceil() as u32;
    for step in 1..steps {
        let (half_sin, half_cos) = (angle * 0.5 * f64::from(step) / f64::from(steps)).sin_cos();
        let sin = 2.0 * half_sin * half_cos;
        // cos(θ) − 1 = −2 sin²(θ/2): не вычитаем близкие координаты центра и радиуса при малых углах.
        let cos_minus_one = -2.0 * half_sin * half_sin;
        push_distinct(
            out,
            point(
                start[0] + radial[0] * cos_minus_one - radial[1] * sin,
                start[1] + radial[0] * sin + radial[1] * cos_minus_one,
            ),
        );
    }
    push_distinct(out, point(v[0] * trim, v[1] * trim));
    true
}

fn push_distinct(out: &mut Vec<Pos2>, point: Pos2) {
    // Соседние дуги могут иметь общую точку касания; индексы явных углов сохраняются при добавлении.
    if out.last() != Some(&point) {
        out.push(point);
    }
}

#[cfg(test)]
mod tests;
