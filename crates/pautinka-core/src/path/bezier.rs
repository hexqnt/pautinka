use emath::{Pos2, Vec2};

use crate::HitPoint;

use super::{PathSample, SAMPLES};

type Point = [f64; 2];

// Инлайнинг открывает обе координаты векторизации, сохраняя формулу и округление emath.
#[inline]
fn lerp_position(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(emath::lerp(a.x..=b.x, t), emath::lerp(a.y..=b.y, t))
}

fn position(controls: &[Pos2], t: f32) -> Pos2 {
    match controls {
        [a, b, c] => lerp_position(lerp_position(*a, *b, t), lerp_position(*b, *c, t), t),
        [a, b, c, d] => {
            let middle = lerp_position(*b, *c, t);
            let left = lerp_position(lerp_position(*a, *b, t), middle, t);
            let right = lerp_position(middle, lerp_position(*c, *d, t), t);
            lerp_position(left, right, t)
        }
        _ => unreachable!("Bézier curves require three or four controls"),
    }
}

pub(super) fn positions(controls: &[Pos2]) -> [Pos2; SAMPLES] {
    std::array::from_fn(|i| position(controls, i as f32 / (SAMPLES - 1) as f32))
}

pub(super) fn sample(controls: &[Pos2], t: f32) -> PathSample {
    PathSample {
        position: position(controls, t),
        tangent: unit_tangent(controls, t),
    }
}

fn unit_tangent(controls: &[Pos2], t: f32) -> Vec2 {
    let mut values = [Vec2::ZERO; 3];
    for (value, pair) in values.iter_mut().zip(controls.windows(2)) {
        *value = pair[1] - pair[0];
    }
    for count in (2..controls.len()).rev() {
        for i in 0..count - 1 {
            values[i] += (values[i + 1] - values[i]) * t;
        }
    }
    crate::math::unit(values[0])
}

fn norm_sq([x, y]: Point) -> f64 {
    x * x + y * y
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

fn split(points: &[Point], t: f64) -> ([Point; 4], [Point; 4]) {
    let mut work = [[0.0; 2]; 4];
    work[..points.len()].copy_from_slice(points);
    let mut left = work;
    let mut right = work;
    for step in 1..points.len() {
        for i in 0..points.len() - step {
            work[i] = lerp(work[i], work[i + 1], t);
        }
        left[step] = work[0];
        right[points.len() - step - 1] = work[points.len() - step - 1];
    }
    (left, right)
}

fn lower_bound_sq(points: &[Point]) -> f64 {
    let mut min = points[0];
    let mut max = points[0];
    for point in &points[1..] {
        for axis in 0..2 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    norm_sq([0.0_f64.clamp(min[0], max[0]), 0.0_f64.clamp(min[1], max[1])])
}

fn nearest(points: &[Point], tolerance_sq: f64, depth: u8, best: &mut f64) {
    let lower = lower_bound_sq(points);
    if lower >= *best || *best <= tolerance_sq || best.sqrt() - lower.sqrt() <= tolerance_sq.sqrt()
    {
        return;
    }
    let (left, right) = split(points, 0.5);
    *best = best.min(norm_sq(right[0]));
    if depth == 24 {
        return;
    }
    let end = points[points.len() - 1];
    let delta = [end[0] - points[0][0], end[1] - points[0][1]];
    let length_sq = norm_sq(delta);
    let flat = points[1..points.len() - 1].iter().all(|p| {
        let t = if length_sq > 0.0 {
            (((p[0] - points[0][0]) * delta[0] + (p[1] - points[0][1]) * delta[1]) / length_sq)
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        let projected = lerp(points[0], end, t);
        norm_sq([p[0] - projected[0], p[1] - projected[1]]) <= tolerance_sq
    });
    if flat {
        let t = if length_sq > 0.0 {
            (-(points[0][0] * delta[0] + points[0][1] * delta[1]) / length_sq).clamp(0.0, 1.0)
        } else {
            0.0
        };
        *best = best.min(norm_sq(split(points, t).1[0]));
        return;
    }
    let count = points.len();
    let (first, second) = if lower_bound_sq(&left[..count]) <= lower_bound_sq(&right[..count]) {
        (&left[..count], &right[..count])
    } else {
        (&right[..count], &left[..count])
    };
    nearest(first, tolerance_sq, depth + 1, best);
    nearest(second, tolerance_sq, depth + 1, best);
}

/// Отсечение выпуклой оболочкой и подразделение de Casteljau; рабочие данные остаются на стеке.
pub(super) fn distance_sq(controls: &[Pos2], point: HitPoint, from: f32, to: f32) -> f64 {
    let [x, y] = point.coordinates();
    let count = controls.len();
    let mut points = [[0.0; 2]; 4];
    for (result, control) in points.iter_mut().zip(controls) {
        *result = [f64::from(control.x) - x, f64::from(control.y) - y];
    }
    let (left, _) = split(&points[..count], f64::from(to));
    let (_, section) = split(&left[..count], f64::from(from) / f64::from(to));
    let section = &section[..count];
    let span = controls
        .windows(2)
        .map(|p| {
            (f64::from(p[0].x) - f64::from(p[1].x)).hypot(f64::from(p[0].y) - f64::from(p[1].y))
        })
        .fold(0.0, f64::max);
    let tolerance_sq = (span * f64::from(f32::EPSILON) * 0.125).powi(2);
    let mut best = norm_sq(section[0]).min(norm_sq(section[count - 1]));
    nearest(section, tolerance_sq, 0, &mut best);
    best
}

#[cfg(test)]
mod tests;
