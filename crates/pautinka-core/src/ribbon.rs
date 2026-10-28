use std::{error::Error, fmt};

use emath::{Pos2, Vec2};

use crate::{EdgePath, PathError, RouteEndpoints, RoutingMode, routing::polyline};

/// Положительное конечное расстояние между соседними дорожками в координатах схемы.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RibbonSpacing(f32);

impl RibbonSpacing {
    pub fn get(self) -> f32 {
        self.0
    }
}

impl Default for RibbonSpacing {
    fn default() -> Self {
        Self(1.0)
    }
}

impl TryFrom<f32> for RibbonSpacing {
    type Error = RibbonSpacingError;
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value.is_finite() && value > 0.0 {
            Ok(Self(value))
        } else {
            Err(RibbonSpacingError)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RibbonSpacingError;
impl Error for RibbonSpacingError {}
impl fmt::Display for RibbonSpacingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Ribbon spacing must be positive and finite")
    }
}
/// Номер дорожки и симметричный размах шлейфа; концы задаются в едином порядке для всей группы.
#[derive(Clone, Copy, Debug)]
pub struct RibbonLane {
    offset: f32,
    extent: f32,
    index: usize,
}

impl RibbonLane {
    pub fn new(index: usize, count: usize, spacing: RibbonSpacing) -> Option<Self> {
        if index >= count {
            return None;
        }
        let extent = (count - 1) as f32 * 0.5 * spacing.get();
        let offset = index as f32 * spacing.get() - extent;
        (offset.is_finite() && extent.is_finite()).then_some(Self {
            offset,
            extent,
            index,
        })
    }
}

impl RoutingMode {
    /// Параллельные сегменты смещаются по нормали; стыки соединяются пересечением смещённых прямых.
    /// На коротких крайних сегментах разводка использует дополнительные разрешённые повороты.
    pub fn route_ribbon(
        self,
        endpoints: RouteEndpoints,
        lane: RibbonLane,
    ) -> Result<EdgePath, PathError> {
        let RouteEndpoints::Between { from, to } = endpoints else {
            return self.route(endpoints, 0.0, lane.index);
        };
        if self == Self::Curved {
            return self.route(endpoints, lane.offset, lane.index);
        }
        if !from.is_finite() || !to.is_finite() {
            return Err(PathError::NonFiniteCoordinate);
        }
        if from == to || lane.extent == 0.0 {
            return self.route(endpoints, 0.0, lane.index);
        }
        let mut points = self.line_points(from, to, 0.0);
        if !points.iter().all(|point| point.is_finite()) {
            return Err(PathError::NonFiniteCoordinate);
        }
        let mut count = 0;
        for point in points {
            if count == 0 || points[count - 1] != point {
                points[count] = point;
                count += 1;
            }
        }
        let points = &points[..count];
        let mut directions = [Vec2::ZERO; 5];
        for (direction, pair) in directions.iter_mut().zip(points.windows(2)) {
            *direction = crate::math::unit(pair[1] - pair[0]);
        }
        let mut shifted = [Pos2::ZERO; 6];
        shifted[0] = from + directions[0].rot90() * lane.offset;
        for (i, triple) in points.windows(3).enumerate() {
            let first = directions[i];
            let second = directions[i + 1];
            let normal_sum = first.rot90() + second.rot90();
            let denominator = 1.0 + first.dot(second);
            if denominator < 0.001 {
                return self.route(endpoints, 0.0, lane.index);
            }
            shifted[i + 1] = triple[1] + normal_sum * (lane.offset / denominator);
        }
        shifted[count - 1] = to + directions[count - 2].rot90() * lane.offset;
        // Общая длина веера для всех дорожек не даёт центральным рёбрам расходиться раньше крайних.
        let fan = lane.extent;
        let start_trim = fan.min(shifted[0].distance(shifted[1]) * 0.25);
        let end_trim = fan.min(shifted[count - 2].distance(shifted[count - 1]) * 0.25);
        shifted[0] += directions[0] * start_trim;
        shifted[count - 1] -= directions[count - 2] * end_trim;
        let start = self.line_points(from, shifted[0], 0.0);
        let end = self.line_points(shifted[count - 1], to, 0.0);
        // Рабочие массивы на стеке; единственная аллокация принадлежит итоговому пути.
        let mut result = [to; 18];
        result[..6].copy_from_slice(&start);
        result[6..6 + count].copy_from_slice(&shifted[..count]);
        result[6 + count..12 + count].copy_from_slice(&end);
        polyline(result)
    }
}

#[cfg(test)]
mod tests;
