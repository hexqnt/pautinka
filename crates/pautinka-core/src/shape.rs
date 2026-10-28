mod geometry;

use std::{error::Error, fmt};

use emath::{Pos2, Rect, Vec2};

use crate::{Anchor, AnchorError, CornerRadius, HitPoint};
use geometry::{CIRCLE_STEPS, Polygon, Vector};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeKind {
    Circle,
    Ellipse,
    Rectangle,
    Triangle,
    Diamond,
    RegularPolygon { sides: u16 },
}

impl ShapeKind {
    fn sides(self) -> Option<u16> {
        match self {
            Self::Circle | Self::Ellipse => None,
            Self::Rectangle | Self::Diamond => Some(4),
            Self::Triangle => Some(3),
            Self::RegularPolygon { sides } => Some(sides),
        }
    }
}

/// Форма и конечные положительные размеры в координатах схемы. Контур вычисляется без выделения памяти.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeShape {
    kind: ShapeKind,
    size: Vec2,
    corner_radius: CornerRadius,
}

// Конструкторы исключают NaN во всех полях.
impl Eq for NodeShape {}

impl NodeShape {
    pub fn circle(radius: f32) -> Result<Self, ShapeError> {
        Self::new(ShapeKind::Circle, Vec2::splat(radius * 2.0))
    }
    pub fn ellipse(size: Vec2) -> Result<Self, ShapeError> {
        Self::new(ShapeKind::Ellipse, size)
    }
    pub fn rectangle(size: Vec2) -> Result<Self, ShapeError> {
        Self::new(ShapeKind::Rectangle, size)
    }
    /// Равнобедренный треугольник с вершиной вверх, вписанный в заданный прямоугольник.
    pub fn triangle(size: Vec2) -> Result<Self, ShapeError> {
        Self::new(ShapeKind::Triangle, size)
    }
    /// Ромб с вершинами в серединах сторон заданного прямоугольника.
    pub fn diamond(size: Vec2) -> Result<Self, ShapeError> {
        Self::new(ShapeKind::Diamond, size)
    }
    /// Правильный многоугольник с 3..=65535 сторонами и вершиной вверх. Радиус описанной окружности задаёт размер; квадрат её границ используется для отсечения.
    pub fn regular_polygon(radius: f32, sides: u16) -> Result<Self, ShapeError> {
        if sides < 3 {
            return Err(ShapeError);
        }
        Self::new(
            ShapeKind::RegularPolygon { sides },
            Vec2::splat(radius * 2.0),
        )
    }
    fn new(kind: ShapeKind, size: Vec2) -> Result<Self, ShapeError> {
        if size.is_finite() && size.x > 0.0 && size.y > 0.0 {
            Ok(Self {
                kind,
                size,
                corner_radius: CornerRadius::default(),
            })
        } else {
            Err(ShapeError)
        }
    }
    /// Скругляет углы касательными круговыми дугами. Отступ от каждой вершины ограничен половиной соседних сторон; для круга и эллипса метод ничего не меняет.
    pub fn with_corner_radius(mut self, radius: CornerRadius) -> Self {
        if !matches!(self.kind, ShapeKind::Circle | ShapeKind::Ellipse) {
            self.corner_radius = radius;
        }
        self
    }
    pub fn kind(self) -> ShapeKind {
        self.kind
    }
    pub fn size(self) -> Vec2 {
        self.size
    }
    /// Запрошенный радиус; фактические дуги могут иметь меньший радиус, чтобы не пересекаться.
    pub fn corner_radius(self) -> CornerRadius {
        self.corner_radius
    }
    pub fn bounds(self, center: Pos2) -> Rect {
        Rect::from_center_size(center, self.size)
    }

    /// Верхняя граница числа точек `outline`, чтобы вызывающий код мог выделить итоговый буфер один раз.
    pub fn outline_capacity(self) -> usize {
        self.kind.sides().map_or(CIRCLE_STEPS as usize, |sides| {
            let sides = usize::from(sides);
            if self.corner_radius.get() == 0.0 {
                sides
            } else {
                // Суммарный поворот выпуклого контура — 360°; добавляем округление числа шагов дуг и точки касания.
                CIRCLE_STEPS as usize + sides * 2
            }
        })
    }
    /// Выход луча из центра на контур фигуры, включая скругления. Направление якоря совпадает с направлением луча.
    pub fn anchor(self, center: Pos2, direction: Vec2) -> Result<Anchor, AnchorError> {
        let anchor = Anchor::new(center, direction)?;
        let direction = Vector::from_vec(anchor.direction()).unit();
        let distance = Polygon::new(self).map_or_else(
            || {
                let half = Vector::from_vec(self.size) * 0.5;
                1.0 / (direction.x / half.x).hypot(direction.y / half.y)
            },
            |polygon| polygon.ray_distance(direction),
        );
        let position = Vector::from_pos(center) + direction * distance;
        Anchor::new(position.to_pos(), anchor.direction())
    }
    pub fn contains(self, center: Pos2, point: Pos2) -> bool {
        self.contains_vector(Vector::from_pos(point) - Vector::from_pos(center))
    }
    /// Сохраняет локальные смещения указателя при больших абсолютных координатах.
    pub fn contains_precise(self, center: Pos2, point: HitPoint) -> bool {
        let [x, y] = point.coordinates();
        self.contains_vector(Vector { x, y } - Vector::from_pos(center))
    }
    /// Точка относительно центра формы; удобна для выбора с экранным допуском.
    pub fn contains_local(self, point: Vec2) -> bool {
        self.contains_vector(Vector::from_vec(point))
    }
    fn contains_vector(self, point: Vector) -> bool {
        if !point.x.is_finite() || !point.y.is_finite() {
            return false;
        }
        Polygon::new(self).map_or_else(
            || {
                let half = Vector::from_vec(self.size) * 0.5;
                (point.x / half.x).hypot(point.y / half.y) <= 1.0 + geometry::EPSILON
            },
            |polygon| polygon.contains(point),
        )
    }
    /// Обходит выпуклый контур по часовой стрелке в экранных координатах относительно центра, без повторения первой точки. Дуги аппроксимируются с шагом не более 5°; ошибка хорды меньше 0,001 радиуса. Память принадлежит вызывающему коду.
    pub fn outline(self, mut visit: impl FnMut(Pos2)) {
        if let Some(polygon) = Polygon::new(self) {
            let mut first = None;
            let mut last = None;
            polygon.outline(|point| {
                let point = point.to_pos();
                if last != Some(point) && (first != Some(point) || last.is_none()) {
                    first.get_or_insert(point);
                    last = Some(point);
                    visit(point);
                }
            });
        } else {
            let half = Vector::from_vec(self.size) * 0.5;
            for step in 0..CIRCLE_STEPS {
                let angle = std::f64::consts::TAU * f64::from(step) / f64::from(CIRCLE_STEPS);
                let (sin, cos) = angle.sin_cos();
                visit(
                    Vector {
                        x: cos * half.x,
                        y: sin * half.y,
                    }
                    .to_pos(),
                );
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShapeError;

impl Error for ShapeError {}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Shape dimensions must be positive and finite; a regular polygon requires at least three sides")
    }
}

#[cfg(test)]
mod tests;
