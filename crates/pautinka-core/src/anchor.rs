use std::{error::Error, fmt};

use crate::{PortId, Pos2, Vec2};

/// Конечная точка подключения с единичным направлением наружу из оборудования.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Anchor {
    position: Pos2,
    direction: Vec2,
}

impl Anchor {
    pub fn new(position: Pos2, direction: Vec2) -> Result<Self, AnchorError> {
        if !position.is_finite() || !direction.is_finite() || direction == Vec2::ZERO {
            return Err(AnchorError);
        }
        let length = direction.length();
        let direction = if length.is_normal() {
            direction / length
        } else {
            // Масштабирование сохраняет единичность при переполнении длины и субнормальных компонентах.
            let scaled = direction / direction.abs().max_elem();
            scaled / scaled.length()
        };
        Ok(Self {
            position,
            direction,
        })
    }
    pub fn position(self) -> Pos2 {
        self.position
    }
    pub fn direction(self) -> Vec2 {
        self.direction
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnchorError;

impl Error for AnchorError {}

impl fmt::Display for AnchorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Anchor position must be finite and direction must be finite and nonzero")
    }
}
/// Порт не владеет предметными данными; его идентичность сохраняется при перемещении якоря.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Port {
    pub id: PortId,
    pub anchor: Anchor,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NodeShape, pos2};

    #[test]
    fn anchors_follow_boundaries_and_reject_invalid_geometry() {
        for scale in [0.0001, 1.0, 10000.0] {
            let center = pos2(10.0, 20.0) * scale;
            let rectangle = NodeShape::rectangle(Vec2::new(20.0, 10.0) * scale).unwrap();
            let anchor = rectangle.anchor(center, Vec2::new(1.0, 1.0)).unwrap();
            assert!(
                anchor
                    .position()
                    .distance(center + Vec2::splat(5.0 * scale))
                    < scale * 0.001
            );
            let circle = NodeShape::circle(5.0 * scale).unwrap();
            assert!(
                (circle
                    .anchor(center, Vec2::Y)
                    .unwrap()
                    .position()
                    .distance(center)
                    - 5.0 * scale)
                    .abs()
                    < scale * 0.001
            );
            assert!(rectangle.contains(center, center));
            assert!(!circle.contains(center, center + Vec2::splat(5.0 * scale)));
        }
        assert!(Anchor::new(Pos2::ZERO, Vec2::ZERO).is_err());
        assert!(Anchor::new(pos2(f32::NAN, 0.0), Vec2::X).is_err());
    }

    #[test]
    fn extreme_finite_directions_remain_normalized() {
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;
        for scale in [
            f32::from_bits(1),
            f32::MIN_POSITIVE * 0.25,
            f32::MIN_POSITIVE,
            1.0,
            f32::MAX,
        ] {
            for (direction, expected) in [
                (Vec2::splat(scale), Vec2::splat(diagonal)),
                (Vec2::new(scale, -scale), Vec2::new(diagonal, -diagonal)),
                (Vec2::new(scale, 0.0), Vec2::X),
                (Vec2::new(0.0, -scale), -Vec2::Y),
            ] {
                let anchor = Anchor::new(Pos2::ZERO, direction).unwrap();
                assert!(anchor.direction().is_normalized());
                assert!((anchor.direction() - expected).length() <= f32::EPSILON);
            }
        }
        for direction in [Vec2::ZERO, Vec2::NAN, Vec2::INFINITY] {
            assert!(Anchor::new(Pos2::ZERO, direction).is_err());
        }
    }
}
