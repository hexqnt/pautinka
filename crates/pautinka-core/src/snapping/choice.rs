use emath::Pos2;

use super::{SnapResult, SnapTarget};

/// Общий кандидат сетки и направляющих; расстояние считается до округления результата в f32.
pub(super) struct Choice {
    pub result: SnapResult,
    pub held: u8,
    pub count: u8,
    pub distance_squared: f64,
}

impl Choice {
    pub fn new(
        position: Pos2,
        point: [f64; 2],
        target: SnapTarget,
        held: u8,
        count: u8,
    ) -> Option<Self> {
        let snapped = Pos2::new(point[0] as f32, point[1] as f32);
        if !snapped.is_finite() {
            return None;
        }
        let dx = point[0] - f64::from(position.x);
        let dy = point[1] - f64::from(position.y);
        Some(Self {
            result: SnapResult {
                position: snapped,
                target: Some(target),
            },
            held,
            count,
            distance_squared: dx * dx + dy * dy,
        })
    }

    /// Удержание важнее расстояния; при равенстве сохраняется другой кандидат.
    pub fn preferred_over(&self, other: &Self) -> bool {
        (self.held > 0)
            .cmp(&(other.held > 0))
            .then_with(|| other.distance_squared.total_cmp(&self.distance_squared))
            .is_gt()
    }
}
