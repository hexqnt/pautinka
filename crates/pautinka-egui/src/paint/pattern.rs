use egui::{Painter, Pos2, Shape, Stroke};

use crate::LinePattern;

#[derive(Clone, Copy)]
enum Run {
    Dash(f64),
    Dot(f64),
    Gap(f64),
}

impl Run {
    fn length(self) -> f64 {
        match self {
            Self::Dash(length) | Self::Dot(length) | Self::Gap(length) => length,
        }
    }
}

/// Проверенный рисунок; размеры точек берутся из основного штриха один раз, до рисования halo.
pub(super) struct Pattern(Kind);

enum Kind {
    Solid,
    Dashed { dash: f32, gap: f32 },
    Dotted([Run; 2]),
    DashDot([Run; 4]),
    DashDotDot([Run; 6]),
}

impl Pattern {
    pub(super) fn new(pattern: LinePattern, width: f32) -> Self {
        let positive = |value: f32| value.is_finite() && value > 0.0;
        let dot = Run::Dot(f64::from(width));
        Self(match pattern {
            LinePattern::Dashed { dash, gap } if positive(dash) && positive(gap) => {
                Kind::Dashed { dash, gap }
            }
            LinePattern::Dotted { gap } if positive(gap) && positive(width) => {
                Kind::Dotted([dot, Run::Gap(f64::from(gap))])
            }
            LinePattern::DashDot { dash, gap } | LinePattern::DashDotDot { dash, gap }
                if positive(dash) && positive(gap) && positive(width) =>
            {
                let dash = Run::Dash(f64::from(dash));
                let gap = Run::Gap(f64::from(gap));
                if matches!(pattern, LinePattern::DashDotDot { .. }) {
                    Kind::DashDotDot([dash, gap, dot, gap, dot, gap])
                } else {
                    Kind::DashDot([dash, gap, dot, gap])
                }
            }
            _ => Kind::Solid,
        })
    }

    pub(super) fn is_solid(&self) -> bool {
        matches!(self.0, Kind::Solid)
    }

    pub(super) fn paint(&self, painter: &Painter, points: &[Pos2], stroke: Stroke) {
        let emit = |shape| {
            painter.add(shape);
        };
        match &self.0 {
            Kind::Solid => {
                if let &[a, b] = points {
                    painter.line_segment([a, b], stroke);
                } else {
                    painter.add(Shape::line(points.to_vec(), stroke));
                }
            }
            Kind::Dashed { dash, gap } => {
                painter.extend(Shape::dashed_line(points, stroke, *dash, *gap));
            }
            Kind::Dotted(runs) => paint_marks(points, runs, stroke, emit),
            Kind::DashDot(runs) => paint_marks(points, runs, stroke, emit),
            Kind::DashDotDot(runs) => paint_marks(points, runs, stroke, emit),
        }
    }
}

fn paint_marks(points: &[Pos2], runs: &[Run], stroke: Stroke, mut emit: impl FnMut(Shape)) {
    let mut segments = points.windows(2).filter_map(|pair| {
        let [a, b] = [pair[0], pair[1]];
        let delta = b - a;
        let length = f64::from(delta.x).hypot(f64::from(delta.y));
        (length > 0.0 && length.is_finite()).then_some((a, delta, length))
    });
    let Some(mut segment) = segments.next() else {
        return;
    };
    let mut offset = 0.0;
    // Один элемент рисунка может пересекать несколько сегментов, не сбрасывая фазу на вершинах.
    for &run in runs.iter().cycle() {
        let mut remaining = run.length();
        loop {
            let (a, delta, length) = segment;
            let point = |distance: f64| a + delta * (distance / length) as f32;
            let step = remaining.min(length - offset);
            match run {
                Run::Dash(_) => emit(Shape::line_segment(
                    [point(offset), point(offset + step)],
                    stroke,
                )),
                Run::Dot(diameter) => {
                    let half = diameter * 0.5;
                    if remaining > half && remaining - step <= half {
                        emit(Shape::circle_filled(
                            point(offset + remaining - half),
                            stroke.width * 0.5,
                            stroke.color,
                        ));
                    }
                }
                Run::Gap(_) => {}
            }
            offset += step;
            remaining -= step;
            if offset >= length {
                let Some(next) = segments.next() else {
                    return;
                };
                segment = next;
                offset = 0.0;
            }
            if remaining == 0.0 {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests;
