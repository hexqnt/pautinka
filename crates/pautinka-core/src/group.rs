use std::{error::Error, fmt};

use emath::{Rect, Vec2};

/// Конечная прямоугольная рамка с заголовком в мировых координатах; принадлежность элементов не хранится.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupFrame {
    bounds: Rect,
    header_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupFrameError {
    InvalidBounds,
    InvalidHeaderHeight,
    InvalidPadding,
    EmptyContent,
}

impl fmt::Display for GroupFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidBounds => {
                "Group bounds must be finite with positive representable dimensions"
            }
            Self::InvalidHeaderHeight => {
                "Group header height must be positive, finite and fit inside the frame"
            }
            Self::InvalidPadding => "Group padding must be finite and nonnegative",
            Self::EmptyContent => "Cannot enclose an empty group",
        })
    }
}
impl Error for GroupFrameError {}

impl GroupFrame {
    pub fn new(bounds: Rect, header_height: f32) -> Result<Self, GroupFrameError> {
        if !bounds.is_finite() || !bounds.size().is_finite() || bounds.size().min_elem() <= 0.0 {
            return Err(GroupFrameError::InvalidBounds);
        }
        if !header_height.is_finite()
            || header_height <= 0.0
            || header_height > bounds.height()
            || bounds.min.y + header_height <= bounds.min.y
        {
            return Err(GroupFrameError::InvalidHeaderHeight);
        }
        Ok(Self {
            bounds,
            header_height,
        })
    }

    /// Вычисляет рамку по подготовленным границам содержимого, добавляя отступы и место для заголовка сверху.
    pub fn enclosing(
        content: impl IntoIterator<Item = Rect>,
        padding: Vec2,
        header_height: f32,
    ) -> Result<Self, GroupFrameError> {
        if !padding.is_finite() || padding.min_elem() < 0.0 {
            return Err(GroupFrameError::InvalidPadding);
        }
        let mut bounds = Rect::NOTHING;
        for rect in content {
            if !rect.is_finite() || !rect.size().is_finite() || rect.size().min_elem() < 0.0 {
                return Err(GroupFrameError::InvalidBounds);
            }
            bounds = bounds.union(rect);
        }
        if bounds == Rect::NOTHING {
            return Err(GroupFrameError::EmptyContent);
        }
        bounds = bounds.expand2(padding);
        bounds.min.y -= header_height;
        Self::new(bounds, header_height)
    }

    pub fn bounds(self) -> Rect {
        self.bounds
    }
    pub fn header(self) -> Rect {
        Rect::from_min_max(
            self.bounds.min,
            emath::pos2(self.bounds.max.x, self.bounds.min.y + self.header_height),
        )
    }
}

#[cfg(test)]
mod tests;
