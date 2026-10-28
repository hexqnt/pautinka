mod bezier;
mod picking;
mod rounded;

use std::{error::Error, fmt};

use emath::{Pos2, Rect, Vec2};

pub use rounded::CornerRadius;

const SAMPLES: usize = 25;

/// Заимствованная геометрия пути в координатах схемы.
#[derive(Clone, Copy, Debug)]
pub enum Curve<'a> {
    Quadratic([Pos2; 3]),
    Cubic([Pos2; 4]),
    Polyline(&'a [Pos2]),
}

#[derive(Clone, Debug)]
enum PathData {
    Line([Pos2; 2]),
    Quadratic {
        controls: [Pos2; 3],
        samples: [Pos2; SAMPLES],
    },
    Cubic {
        controls: [Pos2; 4],
        samples: [Pos2; SAMPLES],
    },
    Polyline {
        points: Box<[Pos2]>,
        unrounded: Option<Box<[Pos2]>>,
        // None: все внутренние вершины угловые; Some: явные углы скруглённого пути.
        corners: Option<Box<[usize]>>,
    },
}

impl PathData {
    fn bezier_controls(&self) -> Option<&[Pos2]> {
        match self {
            Self::Quadratic { controls, .. } => Some(controls),
            Self::Cubic { controls, .. } => Some(controls),
            Self::Line(_) | Self::Polyline { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathError {
    InvalidCornerRadius,
    TooFewPoints,
    NonFiniteCoordinate,
    NonFiniteLength,
}

impl Error for PathError {}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidCornerRadius => "Corner radius must be finite and nonnegative",
            Self::TooFewPoints => "A path requires at least two points",
            Self::NonFiniteCoordinate => "Path coordinates must be finite",
            Self::NonFiniteLength => "Path length exceeds the supported coordinate range",
        })
    }
}

/// Положение и единичное направление вдоль пути; для вырожденного пути направление нулевое.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathSample {
    pub position: Pos2,
    pub tangent: Vec2,
}

/// Общая геометрия для рисования, выбора, подписей, стрелок и маршрутизатора.
///
/// Ломаные хранятся точно. Длины кривых вычисляются по 25 отсчётам; выбор уточняется по настоящей кривой.
/// Кривые и прямые отрезки не выделяют память в куче; скруглённая ломаная дополнительно сохраняет исходные точки для упрощённого отображения.
#[derive(Clone, Debug)]
pub struct EdgePath {
    data: PathData,
    bounds: Rect,
    length: f32,
    midpoint: Pos2,
    arrows: [PathSample; 2],
    deviation_per_step: f32,
    rounding_margin: f32,
}

impl EdgePath {
    /// Совместимый конструктор кривой; точное совпадение концов подразумевает петлю.
    /// Для явной топологии используйте `RoutingMode::route` с `RouteEndpoints`.
    /// Некорректные внешние координаты следует обрабатывать через `try_quadratic` / `try_cubic`.
    pub fn new(from: Pos2, to: Pos2, offset: f32, lane: usize) -> Self {
        let result = if from == to {
            let radius = 45.0 + lane as f32 * 22.0;
            Self::try_cubic([
                from,
                from + Vec2::new(-radius, -radius * 2.0),
                from + Vec2::new(radius, -radius * 2.0),
                to,
            ])
        } else {
            Self::try_offset_quadratic(from, to, offset)
        };
        result.expect("EdgePath::new requires finite coordinates and representable length")
    }

    pub fn try_line(from: Pos2, to: Pos2) -> Result<Self, PathError> {
        Self::build(PathData::Line([from, to]))
    }

    pub fn try_polyline(points: impl Into<Box<[Pos2]>>) -> Result<Self, PathError> {
        let points = points.into();
        if points.len() < 2 {
            return Err(PathError::TooFewPoints);
        }
        Self::build(PathData::Polyline {
            points,
            unrounded: None,
            corners: None,
        })
    }

    pub fn try_quadratic(controls: [Pos2; 3]) -> Result<Self, PathError> {
        let samples = bezier::positions(&controls);
        Self::build(PathData::Quadratic { controls, samples })
    }

    pub(crate) fn try_offset_quadratic(
        from: Pos2,
        to: Pos2,
        offset: f32,
    ) -> Result<Self, PathError> {
        let delta = to - from;
        let normal = if delta == Vec2::ZERO {
            Vec2::ZERO
        } else {
            crate::math::unit(delta).rot90()
        };
        Self::try_quadratic([from, from.lerp(to, 0.5) + normal * offset, to])
    }

    pub fn try_cubic(controls: [Pos2; 4]) -> Result<Self, PathError> {
        let samples = bezier::positions(&controls);
        Self::build(PathData::Cubic { controls, samples })
    }

    fn build(data: PathData) -> Result<Self, PathError> {
        let mut path = Self {
            data,
            bounds: Rect::NOTHING,
            ..Self::default()
        };
        let controls: &[Pos2] = match &path.data {
            PathData::Line(points) => points,
            PathData::Quadratic { controls, .. } => controls,
            PathData::Cubic { controls, .. } => controls,
            PathData::Polyline { points, .. } => points,
        };
        for &point in controls {
            if !point.is_finite() {
                return Err(PathError::NonFiniteCoordinate);
            }
            path.bounds.extend_with(point);
        }
        path.length = path
            .points()
            .windows(2)
            .map(|pair| pair[0].distance(pair[1]))
            .sum();
        if !path.length.is_finite() {
            return Err(PathError::NonFiniteLength);
        }
        if let Some(controls) = path.data.bezier_controls() {
            let degree = (controls.len() - 1) as f32;
            let maximum = controls
                .windows(3)
                .map(|triple| ((triple[2] - triple[1]) - (triple[1] - triple[0])).length())
                .reduce(f32::max)
                .unwrap();
            let second_derivative = degree * (degree - 1.0) * maximum;
            path.deviation_per_step =
                second_derivative / (8.0 * ((SAMPLES - 1) * (SAMPLES - 1)) as f32);
            if !path.deviation_per_step.is_finite() {
                return Err(PathError::NonFiniteLength);
            }
            path.rounding_margin = path
                .bounds
                .min
                .to_vec2()
                .abs()
                .max(path.bounds.max.to_vec2().abs())
                .max_elem()
                * f32::EPSILON
                * 8.0;
        }
        let [reverse, midpoint, forward] = path.sample_many([
            path.arrow_distance(true),
            path.length * 0.5,
            path.arrow_distance(false),
        ]);
        path.midpoint = midpoint.position;
        path.arrows = [
            forward,
            PathSample {
                tangent: reverse.tangent * -1.0,
                ..reverse
            },
        ];
        Ok(path)
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn length(&self) -> f32 {
        self.length
    }
    pub fn midpoint(&self) -> Pos2 {
        self.midpoint
    }

    pub fn curve(&self) -> Curve<'_> {
        match &self.data {
            PathData::Line(points) => Curve::Polyline(points),
            PathData::Quadratic { controls, .. } => Curve::Quadratic(*controls),
            PathData::Cubic { controls, .. } => Curve::Cubic(*controls),
            PathData::Polyline { points, .. } => Curve::Polyline(points),
        }
    }

    /// Геометрия без скругления углов; исходная ломаная заимствуется без копирования, Безье сохраняются.
    pub fn unrounded_curve(&self) -> Curve<'_> {
        match &self.data {
            PathData::Polyline {
                unrounded: Some(points),
                ..
            } => Curve::Polyline(points),
            _ => self.curve(),
        }
    }

    /// Отсчёты кривой либо исходные точки ломаной; срез заимствуется без аллокаций.
    pub fn points(&self) -> &[Pos2] {
        match &self.data {
            PathData::Line(points) => points,
            PathData::Quadratic { samples, .. } | PathData::Cubic { samples, .. } => samples,
            PathData::Polyline { points, .. } => points,
        }
    }

    /// Является ли внутренняя вершина плавным стыком дискретизированной кривой, а не углом маршрута.
    /// Концы пути и вершины обычной ломаной возвращают false.
    pub fn is_smooth_vertex(&self, index: usize) -> bool {
        if index == 0 || index >= self.points().len() - 1 {
            return false;
        }
        match &self.data {
            PathData::Quadratic { .. } | PathData::Cubic { .. } => true,
            PathData::Polyline {
                corners: Some(corners),
                ..
            } => corners.binary_search(&index).is_err(),
            PathData::Line(_) | PathData::Polyline { corners: None, .. } => false,
        }
    }

    pub fn arrow(&self, reverse: bool) -> [Pos2; 2] {
        let sample = self.arrow_sample(reverse);
        [sample.position - sample.tangent, sample.position]
    }

    /// Расстояние от начала пути до стрелки с учётом её направления.
    pub fn arrow_distance(&self, reverse: bool) -> f32 {
        self.length * if reverse { 0.2 } else { 0.8 }
    }

    /// Положение и направление стрелки хранятся раздельно, без потери единичного вектора при больших координатах.
    pub fn arrow_sample(&self, reverse: bool) -> PathSample {
        self.arrows[usize::from(reverse)]
    }

    /// Расстояние ограничивается концами пути; NaN и бесконечность отклоняются.
    pub fn at_distance(&self, distance: f32) -> Option<PathSample> {
        distance
            .is_finite()
            .then(|| self.sample_many([distance.clamp(0.0, self.length)])[0])
    }

    // Каждую дистанцию уменьшаем отдельно, сохраняя округление одиночного запроса.
    fn sample_many<const N: usize>(&self, mut distances: [f32; N]) -> [PathSample; N] {
        let points = self.points();
        let controls = self.data.bezier_controls();
        let mut samples = [PathSample {
            position: points[0],
            tangent: Vec2::ZERO,
        }; N];
        let mut pending = [true; N];
        for (index, pair) in points.windows(2).enumerate() {
            let delta = pair[1] - pair[0];
            let length = delta.length();
            if length == 0.0 {
                continue;
            }
            let last = PathSample {
                position: pair[1],
                tangent: delta / length,
            };
            for ((distance, sample), pending) in
                distances.iter_mut().zip(&mut samples).zip(&mut pending)
            {
                if !*pending {
                    continue;
                }
                *sample = last;
                if *distance <= length {
                    *sample = controls.map_or_else(
                        || PathSample {
                            position: pair[0].lerp(pair[1], *distance / length),
                            tangent: last.tangent,
                        },
                        |controls| {
                            let t = (index as f32 + *distance / length) / (SAMPLES - 1) as f32;
                            bezier::sample(controls, t)
                        },
                    );
                    *pending = false;
                } else {
                    *distance -= length;
                }
            }
            if !pending.contains(&true) {
                return samples;
            }
        }
        samples
    }
}

impl Default for EdgePath {
    fn default() -> Self {
        Self {
            data: PathData::Quadratic {
                controls: [Pos2::ZERO; 3],
                samples: [Pos2::ZERO; SAMPLES],
            },
            bounds: Rect::ZERO,
            length: 0.0,
            midpoint: Pos2::ZERO,
            arrows: [PathSample {
                position: Pos2::ZERO,
                tangent: Vec2::ZERO,
            }; 2],
            deviation_per_step: 0.0,
            rounding_margin: 0.0,
        }
    }
}

#[cfg(test)]
mod tests;
