use std::ops::{Add, Mul, Sub};

use emath::{Pos2, Vec2};

use super::{NodeShape, ShapeKind};

// Допуск соответствует округлению координат контура и якорей в f32; вычисления остаются в f64 для крайних масштабов.
pub(super) const EPSILON: f64 = 4.0 * f32::EPSILON as f64;
pub(super) const CIRCLE_STEPS: u32 = 72;
const ARC_STEP: f64 = std::f64::consts::TAU / CIRCLE_STEPS as f64;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Vector {
    pub x: f64,
    pub y: f64,
}

impl Vector {
    pub fn from_vec(value: Vec2) -> Self {
        Self {
            x: f64::from(value.x),
            y: f64::from(value.y),
        }
    }
    pub fn from_pos(value: Pos2) -> Self {
        Self::from_vec(value.to_vec2())
    }
    pub fn to_pos(self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }
    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
    fn cross(self, other: Self) -> f64 {
        self.x * other.y - self.y * other.x
    }
    fn length(self) -> f64 {
        self.x.hypot(self.y)
    }
    pub fn unit(self) -> Self {
        self * (1.0 / self.length())
    }
    fn rot90(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }
}

impl Add for Vector {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl Sub for Vector {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}
impl Mul<f64> for Vector {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Polygon {
    kind: PolygonKind,
    sides: u16,
    half: Vector,
    radius: f64,
}

#[derive(Clone, Copy)]
enum PolygonKind {
    Rectangle,
    Triangle,
    Diamond,
    Regular,
}

impl Polygon {
    pub fn new(shape: NodeShape) -> Option<Self> {
        let kind = match shape.kind() {
            ShapeKind::Circle | ShapeKind::Ellipse => return None,
            ShapeKind::Rectangle => PolygonKind::Rectangle,
            ShapeKind::Triangle => PolygonKind::Triangle,
            ShapeKind::Diamond => PolygonKind::Diamond,
            ShapeKind::RegularPolygon { .. } => PolygonKind::Regular,
        };
        Some(Self {
            kind,
            sides: shape.kind().sides()?,
            half: Vector::from_vec(shape.size()) * 0.5,
            radius: f64::from(shape.corner_radius().get()),
        })
    }
    fn vertex(self, index: u16) -> Vector {
        let (x, y) = match self.kind {
            PolygonKind::Rectangle => {
                [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][usize::from(index)]
            }
            PolygonKind::Triangle => [(0.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][usize::from(index)],
            PolygonKind::Diamond => {
                [(0.0, -1.0), (1.0, 0.0), (0.0, 1.0), (-1.0, 0.0)][usize::from(index)]
            }
            PolygonKind::Regular => {
                let angle = std::f64::consts::TAU * f64::from(index) / f64::from(self.sides);
                let (sin, cos) = angle.sin_cos();
                (sin, -cos)
            }
        };
        Vector {
            x: x * self.half.x,
            y: y * self.half.y,
        }
    }
    fn corners(self) -> impl Iterator<Item = Corner> {
        let first = self.vertex(0);
        let mut previous = self.vertex(self.sides - 1);
        let mut vertex = first;
        (1..self.sides)
            .map(move |index| self.vertex(index))
            .chain(std::iter::once(first))
            .map(move |next| {
                let corner = Corner::new(previous, vertex, next, self.radius);
                previous = vertex;
                vertex = next;
                corner
            })
    }
    pub fn contains(self, point: Vector) -> bool {
        if point.x.abs() > self.half.x * (1.0 + EPSILON)
            || point.y.abs() > self.half.y * (1.0 + EPSILON)
        {
            return false;
        }
        if matches!(self.kind, PolygonKind::Rectangle) && self.radius == 0.0 {
            return true;
        }
        for corner in self.corners() {
            let edge = corner.vertex - corner.previous;
            let tolerance = EPSILON * (edge.x.abs() * self.half.y + edge.y.abs() * self.half.x);
            if edge.cross(point - corner.previous) < -tolerance {
                return false;
            }
            if corner.outside_arc(point) {
                return false;
            }
        }
        true
    }
    pub fn ray_distance(self, direction: Vector) -> f64 {
        let mut distance = f64::INFINITY;
        let mut previous_end = Corner::new(
            self.vertex(self.sides - 2),
            self.vertex(self.sides - 1),
            self.vertex(0),
            self.radius,
        )
        .end;
        for corner in self.corners() {
            let a = previous_end;
            let edge = corner.start - a;
            let denominator = direction.cross(edge);
            if denominator != 0.0 {
                let t = a.cross(edge) / denominator;
                let s = a.cross(direction) / denominator;
                if t >= 0.0 && (-EPSILON..=1.0 + EPSILON).contains(&s) {
                    distance = distance.min(t);
                }
            }
            if corner.radius > 0.0 {
                // Уравнение выражено через точку касания: не вычитаем квадрат большого радиуса из квадрата близкого центра.
                let along = corner.start.dot(direction)
                    - corner.radius * corner.start_normal.dot(direction);
                let constant = corner.start.dot(corner.start)
                    - 2.0 * corner.radius * corner.start.dot(corner.start_normal);
                let discriminant = along * along - constant;
                if discriminant >= 0.0 {
                    let q = along + discriminant.sqrt().copysign(along);
                    for t in [q, constant / q] {
                        if t >= 0.0 && corner.in_sector(direction * t) {
                            distance = distance.min(t);
                        }
                    }
                }
            }
            previous_end = corner.end;
        }
        distance
    }
    pub fn outline(self, mut visit: impl FnMut(Vector)) {
        for corner in self.corners() {
            visit(corner.start);
            if corner.radius > 0.0 {
                let turn = corner
                    .start_normal
                    .cross(corner.end_normal)
                    .max(0.0)
                    .atan2(corner.start_normal.dot(corner.end_normal).clamp(-1.0, 1.0));
                #[allow(clippy::cast_sign_loss)]
                let steps = (turn / ARC_STEP).ceil() as u32;
                for step in 1..steps {
                    let half_angle = turn * 0.5 * f64::from(step) / f64::from(steps);
                    let (half_sin, half_cos) = half_angle.sin_cos();
                    // При почти прямом угле и большом радиусе сохраняем малую стрелу дуги: cos(θ) − 1 = −2 sin²(θ/2).
                    visit(
                        corner.start
                            + corner.start_normal * (-2.0 * corner.radius * half_sin * half_sin)
                            + corner.start_normal.rot90()
                                * (2.0 * corner.radius * half_sin * half_cos),
                    );
                }
                visit(corner.end);
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Corner {
    previous: Vector,
    vertex: Vector,
    start: Vector,
    end: Vector,
    radius: f64,
    start_normal: Vector,
    end_normal: Vector,
}

impl Corner {
    fn new(previous: Vector, vertex: Vector, next: Vector, radius: f64) -> Self {
        if radius == 0.0 {
            return Self {
                previous,
                vertex,
                start: vertex,
                end: vertex,
                radius: 0.0,
                start_normal: Vector::default(),
                end_normal: Vector::default(),
            };
        }
        let incoming = vertex - previous;
        let outgoing = next - vertex;
        let before = incoming.length();
        let after = outgoing.length();
        let u = incoming * (1.0 / before);
        let v = outgoing * (1.0 / after);
        let cross = u.cross(v).max(0.0);
        let dot = u.dot(v).clamp(-1.0, 1.0);
        let tangent = if dot >= 0.0 {
            cross / (1.0 + dot)
        } else {
            (1.0 - dot) / cross
        };
        let trim = (radius * tangent).min(before * 0.5).min(after * 0.5);
        let radius = if tangent > 0.0 { trim / tangent } else { 0.0 };
        let start = vertex - u * trim;
        Self {
            previous,
            vertex,
            start,
            end: vertex + v * trim,
            radius,
            start_normal: u.rot90() * -1.0,
            end_normal: v.rot90() * -1.0,
        }
    }
    fn in_sector(self, point: Vector) -> bool {
        let delta = point - self.start;
        let turn = self.radius * self.start_normal.cross(self.end_normal);
        let start_tolerance = EPSILON
            * ((self.start_normal.x * delta.y).abs() + (self.start_normal.y * delta.x).abs());
        let end_tolerance = EPSILON
            * ((delta.x * self.end_normal.y).abs()
                + (delta.y * self.end_normal.x).abs()
                + turn.abs());
        self.start_normal.cross(delta) >= -start_tolerance
            && delta.cross(self.end_normal) + turn >= -end_tolerance
    }
    fn outside_arc(self, point: Vector) -> bool {
        if self.radius == 0.0 || !self.in_sector(point) {
            return false;
        }
        let delta = point - self.start;
        let squared = delta.dot(delta);
        let projection = 2.0 * self.radius * delta.dot(self.start_normal);
        let tolerance = EPSILON
            * (squared
                + 2.0
                    * ((delta.x.abs() + self.radius * self.start_normal.x.abs())
                        * point.x.abs().max(self.start.x.abs())
                        + (delta.y.abs() + self.radius * self.start_normal.y.abs())
                            * point.y.abs().max(self.start.y.abs())));
        squared + projection > tolerance
    }
}
