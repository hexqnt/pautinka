use emath::{Pos2, Vec2};

use crate::{Anchor, EdgePath, PathError};

/// Геометрический стиль маршрута без обхода препятствий.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoutingMode {
    #[default]
    Curved,
    /// Прямой отрезок между центрами; смещение параллельной дорожки не применяется.
    Straight,
    Orthogonal,
    Octilinear,
    /// Направления сегментов кратны 15° относительно осей схемы.
    Angular15,
}

impl RoutingMode {
    /// Смещение дорожки задаётся в мировых координатах; номер различает размеры петель.
    pub fn route(
        self,
        endpoints: RouteEndpoints,
        offset: f32,
        lane: usize,
    ) -> Result<EdgePath, PathError> {
        self.route_with(endpoints, offset, lane, LoopOptions::default())
    }

    pub fn route_with(
        self,
        endpoints: RouteEndpoints,
        offset: f32,
        lane: usize,
        loops: LoopOptions,
    ) -> Result<EdgePath, PathError> {
        if !offset.is_finite() {
            return Err(PathError::NonFiniteCoordinate);
        }
        match endpoints {
            RouteEndpoints::Loop { position } => {
                let radius = loops.radius + lane as f32 * loops.lane_spacing;
                let point = |x, y| position + loops.direction.rot90() * x - loops.direction * y;
                if matches!(self, Self::Curved | Self::Straight) {
                    EdgePath::try_cubic([
                        position,
                        point(-radius, -radius * 2.0),
                        point(radius, -radius * 2.0),
                        position,
                    ])
                } else if self == Self::Orthogonal {
                    polyline([
                        position,
                        point(0.0, -radius * 2.0),
                        point(radius, -radius * 2.0),
                        point(radius, 0.0),
                        position,
                    ])
                } else {
                    polyline([
                        position,
                        point(-radius, -radius),
                        point(-radius, -radius * 1.5),
                        point(-radius * 0.5, -radius * 2.0),
                        point(radius * 0.5, -radius * 2.0),
                        point(radius, -radius * 1.5),
                        point(radius, -radius),
                        position,
                    ])
                }
            }
            RouteEndpoints::Between { from, to } => {
                if !from.is_finite() || !to.is_finite() {
                    return Err(PathError::NonFiniteCoordinate);
                }
                if self == Self::Straight {
                    return EdgePath::try_line(from, to);
                }
                if self == Self::Curved {
                    return EdgePath::try_offset_quadratic(from, to, offset);
                }
                polyline(self.line_points(from, to, offset))
            }
        }
    }

    /// Соединяет порты с выходом вдоль их наружных направлений; препятствия остаются ответственностью владельца.
    /// Для кривой `lead` задаёт расстояние до контрольных точек, а `offset` не применяется; для ломаной `offset` смещает центральный участок.
    pub fn route_anchors(
        self,
        from: Anchor,
        to: Anchor,
        lead: f32,
        offset: f32,
    ) -> Result<EdgePath, PathError> {
        if !lead.is_finite() || lead < 0.0 || !offset.is_finite() {
            return Err(PathError::NonFiniteLength);
        }
        let start = from.position() + from.direction() * lead;
        let end = to.position() + to.direction() * lead;
        if self == Self::Curved {
            return EdgePath::try_cubic([from.position(), start, end, to.position()]);
        }
        let points = self.line_points(start, end, offset);
        polyline([
            from.position(),
            points[0],
            points[1],
            points[2],
            points[3],
            points[4],
            points[5],
            to.position(),
        ])
    }
    pub(super) fn line_points(self, from: Pos2, to: Pos2, offset: f32) -> [Pos2; 6] {
        if self == Self::Straight {
            return [from, from, from, to, to, to];
        }
        let delta = to - from;
        let horizontal = delta.x.abs() >= delta.y.abs();
        let (major, minor) = if horizontal {
            (delta.x, delta.y)
        } else {
            (delta.y, delta.x)
        };
        let direction = if major < 0.0 { -1.0 } else { 1.0 };
        let length = major.abs();
        let shoulder = match self {
            Self::Curved | Self::Straight => unreachable!(),
            Self::Orthogonal => length * 0.5,
            Self::Octilinear => (length - minor.abs()) * 0.5,
            Self::Angular15 => {
                // В секторе 0–45° выбираем ближайший больший разрешённый наклон; остаток делим между осевыми участками.
                const TAN_15: f32 = 0.267_949_2;
                const TAN_30: f32 = 0.577_350_26;
                let rise = minor.abs();
                let slope = if rise <= length * TAN_15 {
                    TAN_15
                } else if rise <= length * TAN_30 {
                    TAN_30
                } else {
                    1.0
                };
                (length - rise / slope).max(0.0) * 0.5
            }
        };
        let shift = if horizontal {
            Vec2::new(0.0, offset)
        } else {
            Vec2::new(offset, 0.0)
        };
        let from_shifted = from + shift;
        let to_shifted = to + shift;
        let step = if horizontal {
            Vec2::new(shoulder * direction, 0.0)
        } else {
            Vec2::new(0.0, shoulder * direction)
        };
        let (first, second) = if self == Self::Orthogonal {
            let middle = from.lerp(to, 0.5);
            if horizontal {
                (
                    Pos2::new(middle.x, from_shifted.y),
                    Pos2::new(middle.x, to_shifted.y),
                )
            } else {
                (
                    Pos2::new(from_shifted.x, middle.y),
                    Pos2::new(to_shifted.x, middle.y),
                )
            }
        } else {
            (from_shifted + step, to_shifted - step)
        };
        [from, from_shifted, first, second, to_shifted, to]
    }
}

/// Топологическая петля задаётся явно: совпадение координат не создаёт петлю.
#[derive(Clone, Copy, Debug)]
pub enum RouteEndpoints {
    Between { from: Pos2, to: Pos2 },
    Loop { position: Pos2 },
}

/// Размер и направление петель не зависят от единиц измерения приложения.
#[derive(Clone, Copy, Debug)]
pub struct LoopOptions {
    radius: f32,
    lane_spacing: f32,
    direction: Vec2,
}

impl LoopOptions {
    pub fn new(radius: f32, lane_spacing: f32, direction: Vec2) -> Result<Self, PathError> {
        if !radius.is_finite() || radius <= 0.0 || !lane_spacing.is_finite() || lane_spacing < 0.0 {
            return Err(PathError::NonFiniteLength);
        }
        let direction = Anchor::new(Pos2::ZERO, direction)
            .map_err(|_| PathError::NonFiniteCoordinate)?
            .direction();
        Ok(Self {
            radius,
            lane_spacing,
            direction,
        })
    }
}

impl Default for LoopOptions {
    fn default() -> Self {
        Self {
            radius: 45.0,
            lane_spacing: 22.0,
            direction: -Vec2::Y,
        }
    }
}

pub(super) fn polyline<const N: usize>(points: [Pos2; N]) -> Result<EdgePath, PathError> {
    if !points.iter().all(|point| point.is_finite()) {
        return Err(PathError::NonFiniteCoordinate);
    }
    let mut compact = [Pos2::ZERO; N];
    let mut count = 0;
    for point in points {
        if count > 0 && compact[count - 1] == point {
            continue;
        }
        while count >= 2 {
            let a = compact[count - 1] - compact[count - 2];
            let b = point - compact[count - 1];
            if a.x * b.y != a.y * b.x || a.dot(b) < 0.0 {
                break;
            }
            count -= 1;
        }
        compact[count] = point;
        count += 1;
    }
    match &compact[..count] {
        [point] => EdgePath::try_line(*point, *point),
        [from, to] => EdgePath::try_line(*from, *to),
        // Рабочие точки лежат на стеке; память выделяется только для итоговой ломаной.
        points => EdgePath::try_polyline(Box::<[Pos2]>::from(points)),
    }
}

#[cfg(test)]
mod tests;
