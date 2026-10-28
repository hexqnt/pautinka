use egui::Pos2;

use crate::geometry::EdgePath;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Intersection {
    pub fraction: f32,
    pub other_fraction: f32,
    pub sine: f32,
}

pub(super) fn segment_at(points: &[Pos2], index: usize) -> Option<[Pos2; 2]> {
    points.get(index..)?.first_chunk().copied()
}

// Плавный стык принадлежит начинающемуся в нём сегменту: соседний не дублирует пересечение.
pub(super) fn inside_segment(fraction: f64, smooth_start: bool) -> bool {
    (0.0 < fraction || (smooth_start && fraction == 0.0)) && fraction < 1.0
}

/// Пересечения внутренних частей пути, включая плавные стыки; параметры не зависят от масштаба камеры.
pub(super) fn intersection(
    [a, b]: [Pos2; 2],
    [c, d]: [Pos2; 2],
    smooth_starts: [bool; 2],
) -> Option<Intersection> {
    let delta = |a: Pos2, b: Pos2| {
        [
            f64::from(a.x) - f64::from(b.x),
            f64::from(a.y) - f64::from(b.y),
        ]
    };
    let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    let first = delta(b, a);
    let second = delta(d, c);
    let determinant = cross(first, second);
    let t = cross(delta(c, a), second) / determinant;
    let u = cross(delta(c, a), first) / determinant;
    if !inside_segment(t, smooth_starts[0]) || !inside_segment(u, smooth_starts[1]) {
        return None;
    }
    let first_length = first[0].hypot(first[1]);
    let second_length = second[0].hypot(second[1]);
    let sine = determinant.abs() / (first_length * second_length);
    if !sine.is_finite() || sine < 1e-6 {
        return None;
    }
    Some(Intersection {
        fraction: t as f32,
        other_fraction: u as f32,
        sine: sine as f32,
    })
}

/// Отступ вдоль пути до ближайшего настоящего угла или конца; стыки дуг не ограничивают оформление.
pub(super) fn section_clearance(path: &EdgePath, segment: usize, fraction: f32) -> f32 {
    let points = path.points();
    let [a, b] = segment_at(points, segment).expect("Crossing segment exists");
    let length = a.distance(b);
    let before = points[..=segment]
        .windows(2)
        .enumerate()
        .rev()
        .take_while(|(index, _)| path.is_smooth_vertex(index + 1))
        .fold(fraction * length, |distance, (_, pair)| {
            distance + pair[0].distance(pair[1])
        });
    let after = points[segment + 1..]
        .windows(2)
        .enumerate()
        .take_while(|(offset, _)| path.is_smooth_vertex(segment + 1 + offset))
        .fold((1.0 - fraction) * length, |distance, (_, pair)| {
            distance + pair[0].distance(pair[1])
        });
    before.min(after)
}
