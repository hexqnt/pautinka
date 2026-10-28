use egui::Pos2;

use crate::geometry::{Curve, EdgePath};

use super::geometry::{intersection, segment_at};

type Point = [f64; 2];

const MAX_NEWTON_STEPS: usize = 8;

#[derive(Clone, Copy)]
pub(super) struct PathPosition {
    pub segment: usize,
    pub fraction: f32,
}

impl PathPosition {
    pub fn distance(self, path: &EdgePath) -> f32 {
        let points = path.points();
        let before = points[..=self.segment]
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum::<f32>();
        let [a, b] = segment_at(points, self.segment).expect("Crossing segment exists");
        before + self.fraction * a.distance(b)
    }

    fn refined(path: &EdgePath, segment: usize, fraction: f64) -> Option<Self> {
        if matches!(path.curve(), Curve::Polyline(_)) {
            return super::geometry::inside_segment(fraction, path.is_smooth_vertex(segment))
                .then_some(Self {
                    segment,
                    fraction: fraction as f32,
                });
        }
        let position = segment as f64 + fraction;
        let end = (path.points().len() - 1) as f64;
        // Не позволяем уточнению перескочить к далёкому корню.
        if !((-1.0..=2.0).contains(&fraction) && 0.0 < position && position < end) {
            return None;
        }
        // Положительность и верхняя граница проверены выше.
        #[allow(clippy::cast_sign_loss)]
        let segment = position.floor() as usize;
        Some(Self {
            segment,
            fraction: (position - segment as f64) as f32,
        })
    }
}

pub(super) struct PathIntersection {
    pub first: PathPosition,
    pub second: PathPosition,
    pub sine: f32,
}

struct Sample {
    position: Point,
    tangent: Point,
}

enum Controls {
    Line([Point; 2]),
    Quadratic([Point; 3]),
    Cubic([Point; 4]),
}

/// Контрольные точки преобразуются в f64 один раз, до итераций Ньютона.
struct CurveSegment {
    controls: Controls,
    origin: f64,
    scale: f64,
}

impl CurveSegment {
    fn new(path: &EdgePath, segment: usize) -> Self {
        let point = |p: Pos2| [f64::from(p.x), f64::from(p.y)];
        let controls = match path.curve() {
            Curve::Polyline(points) => {
                return Self {
                    controls: Controls::Line(
                        segment_at(points, segment)
                            .expect("Crossing segment exists")
                            .map(point),
                    ),
                    origin: 0.0,
                    scale: 1.0,
                };
            }
            Curve::Quadratic(points) => Controls::Quadratic(points.map(point)),
            Curve::Cubic(points) => Controls::Cubic(points.map(point)),
        };
        Self {
            controls,
            origin: segment as f64,
            scale: 1.0 / (path.points().len() - 1) as f64,
        }
    }

    fn evaluate(&self, fraction: f64) -> Sample {
        let t = (self.origin + fraction) * self.scale;
        let mut sample = match &self.controls {
            Controls::Line(points) => evaluate(*points, t),
            Controls::Quadratic(points) => evaluate(*points, t),
            Controls::Cubic(points) => evaluate(*points, t),
        };
        // Производная по локальной доле сегмента, а не по параметру целой кривой.
        sample.tangent = sample.tangent.map(|value| value * self.scale);
        sample
    }
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    std::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * t)
}

fn evaluate<const N: usize>(mut controls: [Point; N], t: f64) -> Sample {
    for remaining in (3..=N).rev() {
        for index in 0..remaining - 1 {
            controls[index] = lerp(controls[index], controls[index + 1], t);
        }
    }
    Sample {
        position: lerp(controls[0], controls[1], t),
        tangent: std::array::from_fn(|axis| {
            (controls[1][axis] - controls[0][axis]) * (N - 1) as f64
        }),
    }
}

pub(super) fn horizontal(path: &EdgePath, position: PathPosition) -> bool {
    if let Curve::Polyline(points) = path.curve() {
        let [a, b] = segment_at(points, position.segment).expect("Crossing segment exists");
        return (b.x - a.x).abs() >= (b.y - a.y).abs();
    }
    let tangent = CurveSegment::new(path, position.segment)
        .evaluate(f64::from(position.fraction))
        .tangent;
    tangent[0].abs() >= tangent[1].abs()
}

/// Хорды дают начальное приближение; метод Ньютона уточняет центр и угол на настоящих кривых.
pub(super) fn refined_intersection(
    first: &EdgePath,
    segment: usize,
    second: &EdgePath,
    other_segment: usize,
) -> Option<PathIntersection> {
    let hit = intersection(
        segment_at(first.points(), segment)?,
        segment_at(second.points(), other_segment)?,
        [
            first.is_smooth_vertex(segment),
            second.is_smooth_vertex(other_segment),
        ],
    )?;
    if matches!(
        (first.curve(), second.curve()),
        (Curve::Polyline(_), Curve::Polyline(_))
    ) {
        return Some(PathIntersection {
            first: PathPosition {
                segment,
                fraction: hit.fraction,
            },
            second: PathPosition {
                segment: other_segment,
                fraction: hit.other_fraction,
            },
            sine: hit.sine,
        });
    }
    let first_curve = CurveSegment::new(first, segment);
    let second_curve = CurveSegment::new(second, other_segment);
    let mut t = f64::from(hit.fraction);
    let mut u = f64::from(hit.other_fraction);
    let cross = |a: Point, b: Point| a[0] * b[1] - a[1] * b[0];
    for _ in 0..MAX_NEWTON_STEPS {
        let a = first_curve.evaluate(t);
        let b = second_curve.evaluate(u);
        let delta = [b.position[0] - a.position[0], b.position[1] - a.position[1]];
        let determinant = cross(a.tangent, b.tangent);
        if determinant == 0.0 {
            return None;
        }
        t += cross(delta, b.tangent) / determinant;
        u += cross(delta, a.tangent) / determinant;
    }
    let a = first_curve.evaluate(t);
    let b = second_curve.evaluate(u);
    let first_length = a.tangent[0].hypot(a.tangent[1]);
    let second_length = b.tangent[0].hypot(b.tangent[1]);
    let sine = cross(a.tangent, b.tangent).abs() / (first_length * second_length);
    let tolerance = first_length.max(second_length) * 1e-6;
    let error = (a.position[0] - b.position[0]).hypot(a.position[1] - b.position[1]);
    if !sine.is_finite() || sine < 1e-6 || error > tolerance {
        return None;
    }
    Some(PathIntersection {
        first: PathPosition::refined(first, segment, t)?,
        second: PathPosition::refined(second, other_segment, u)?,
        sine: sine as f32,
    })
}

/// Параметр Безье по той же таблице длин, которую использует `EdgePath::at_distance`.
fn parameter(path: &EdgePath, mut distance: f32) -> f32 {
    if distance <= 0.0 {
        return 0.0;
    }
    if distance >= path.length() {
        return 1.0;
    }
    let points = path.points();
    for (index, pair) in points.windows(2).enumerate() {
        let length = pair[0].distance(pair[1]);
        if length == 0.0 {
            continue;
        }
        if distance <= length {
            return (index as f32 + distance / length) / (points.len() - 1) as f32;
        }
        distance -= length;
    }
    1.0
}

fn split<const N: usize>(mut points: [Pos2; N], t: f32) -> ([Pos2; N], [Pos2; N]) {
    let mut left = points;
    let mut right = points;
    for step in 1..N {
        for index in 0..N - step {
            points[index] = points[index].lerp(points[index + 1], t);
        }
        left[step] = points[0];
        right[N - step - 1] = points[N - step - 1];
    }
    (left, right)
}

fn section<const N: usize>(points: [Pos2; N], from: f32, to: f32) -> [Pos2; N] {
    let (left, _) = split(points, to);
    split(left, if to > 0.0 { from / to } else { 0.0 }).1
}

/// Подкривая сохраняет степень и точную форму Безье; дискретизация нужна только для таблицы длин.
pub(super) fn section_at_distance(path: &EdgePath, from: f32, to: f32) -> Curve<'_> {
    let from = parameter(path, from).clamp(0.0, 1.0);
    let to = parameter(path, to).clamp(from, 1.0);
    match path.curve() {
        Curve::Quadratic(points) => Curve::Quadratic(section(points, from, to)),
        Curve::Cubic(points) => Curve::Cubic(section(points, from, to)),
        Curve::Polyline(_) => unreachable!("Curve sections require a Bézier path"),
    }
}
