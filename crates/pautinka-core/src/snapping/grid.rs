use emath::Pos2;

use crate::AlignmentSnap;

use super::{SnapResult, SnapTarget, choice::Choice};

/// Регулярная сетка с началом в `(0, 0)` и положительным конечным мировым шагом.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapGrid {
    step: f64,
}

impl SnapGrid {
    pub fn new(step: f64) -> Option<Self> {
        (step.is_finite() && step > 0.0).then_some(Self { step })
    }

    pub fn step(self) -> f64 {
        self.step
    }

    /// Захватывает ближайшее пересечение целиком; равноудалённые деления округляются от нуля.
    /// Допуски заданы в мировых единицах; прежняя цель удерживается до порога отпускания.
    pub fn snap(
        self,
        position: Pos2,
        tolerance: AlignmentSnap,
        previous: Option<SnapResult>,
    ) -> SnapResult {
        self.candidate(position, tolerance, previous).map_or(
            SnapResult {
                position,
                target: None,
            },
            |candidate| candidate.result,
        )
    }

    pub(super) fn candidate(
        self,
        position: Pos2,
        tolerance: AlignmentSnap,
        previous: Option<SnapResult>,
    ) -> Option<Choice> {
        if !position.is_finite() {
            return None;
        }
        if let Some(previous) =
            previous.filter(|previous| previous.target == Some(SnapTarget::Grid))
            && let Some(point) = self.nearest(previous.position)
            && Pos2::new(point[0] as f32, point[1] as f32) == previous.position
            && let Some(candidate) = capture(position, point, tolerance.release_tolerance(), true)
        {
            return Some(candidate);
        }
        capture(
            position,
            self.nearest(position)?,
            tolerance.tolerance(),
            false,
        )
    }

    fn nearest(self, position: Pos2) -> Option<[f64; 2]> {
        if !position.is_finite() {
            return None;
        }
        Some([position.x, position.y].map(|coordinate| {
            let coordinate = f64::from(coordinate);
            let index = coordinate / self.step;
            // При переполнении номера деления шаг уже меньше точности самой координаты.
            if index.is_finite() {
                index.round() * self.step
            } else {
                coordinate
            }
        }))
    }
}

fn capture(position: Pos2, point: [f64; 2], tolerance: f32, held: bool) -> Option<Choice> {
    let choice = Choice::new(position, point, SnapTarget::Grid, u8::from(held), 2)?;
    (choice.distance_squared <= f64::from(tolerance).powi(2)).then_some(choice)
}

#[cfg(test)]
mod tests;
