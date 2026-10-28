use egui::{Color32, Rect, Stroke, pos2};

use crate::{RenderContext, SnapGrid};

const SUBDIVISIONS: usize = 5;
const GRID_CYCLE: usize = SUBDIVISIONS * SUBDIVISIONS;
const MAX_AXIS_INTERVALS: f64 = 512.0;
const MAX_DOTS: f64 = 16_384.0;

/// Представление адаптивной сетки; оба режима используют одинаковые мировые координаты.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridKind {
    #[default]
    Lines,
    Dots,
}

/// Шаг магнитной привязки; видимость рисунка сетки не влияет на выбранный режим.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GridSnapMode {
    #[default]
    Disabled,
    /// Тот же мировой шаг, который выбран для текущего масштаба рисунка.
    Adaptive,
    /// Базовый мировой шаг независимо от масштаба камеры.
    Fixed,
}

// Пересечение получает более мелкий из уровней двух осей.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum GridRank {
    Minor,
    Middle,
    Major,
}

/// Шаги сетки образуют ряд `base_step × 5ⁿ`; каждое пятое деление выделяется.
/// Мелкие деления плавно исчезают при отдалении. Привязка включается отдельно через `GridSnapMode`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridOptions {
    /// Отображение рисунка; скрытая сетка сохраняет параметры привязки.
    pub visible: bool,
    pub kind: GridKind,
    pub minor_color: Color32,
    pub major_color: Color32,
    base_step: f32,
    min_spacing: f32,
    minor_width: f32,
    major_width: f32,
}

impl GridOptions {
    /// Нижняя граница экранного интервала ограничивает плотность сетки.
    pub const MIN_SPACING: f32 = 8.0;

    /// Базовый шаг задан в мировых единицах, минимальный экранный интервал — в точках, не меньше восьми.
    pub fn new(base_step: f32, min_spacing: f32) -> Option<Self> {
        Self::default().with_spacing(base_step, min_spacing)
    }

    /// Меняет шаги, сохраняя вид, цвета и размеры делений; ограничения совпадают с `new`.
    pub fn with_spacing(self, base_step: f32, min_spacing: f32) -> Option<Self> {
        (base_step.is_finite()
            && base_step > 0.0
            && min_spacing.is_finite()
            && min_spacing >= Self::MIN_SPACING)
            .then_some(Self {
                base_step,
                min_spacing,
                ..self
            })
    }

    pub fn base_step(self) -> f32 {
        self.base_step
    }

    pub fn min_spacing(self) -> f32 {
        self.min_spacing
    }

    /// Положительные конечные размеры в экранных точках: толщина линий либо радиус точек.
    pub fn with_widths(self, minor: f32, major: f32) -> Option<Self> {
        (minor.is_finite() && minor > 0.0 && major.is_finite() && major > 0.0).then_some(Self {
            minor_width: minor,
            major_width: major,
            ..self
        })
    }

    pub fn minor_width(self) -> f32 {
        self.minor_width
    }

    pub fn major_width(self) -> f32 {
        self.major_width
    }

    pub(crate) fn snap_grid(
        self,
        mode: GridSnapMode,
        zoom: f32,
        viewport: Rect,
    ) -> Option<SnapGrid> {
        let step = match mode {
            GridSnapMode::Disabled => return None,
            GridSnapMode::Adaptive => GridLevel::new(&self, zoom, viewport).world_step,
            GridSnapMode::Fixed => f64::from(self.base_step),
        };
        SnapGrid::new(step)
    }
}

impl Default for GridOptions {
    fn default() -> Self {
        Self {
            visible: true,
            kind: GridKind::Lines,
            minor_color: Color32::from_white_alpha(12),
            major_color: Color32::from_white_alpha(30),
            base_step: 10.0,
            min_spacing: 16.0,
            minor_width: 1.0,
            major_width: 1.5,
        }
    }
}

struct GridStrokes {
    minor: Stroke,
    middle: Stroke,
    major: Stroke,
}

impl GridStrokes {
    fn new(options: &GridOptions, fade: f32) -> Self {
        // Три вложенных уровня сохраняют цвет и толщину делений при смене масштаба.
        Self {
            minor: Stroke::new(
                options.minor_width,
                options.minor_color.gamma_multiply(fade),
            ),
            middle: Stroke::new(
                egui::lerp(options.minor_width..=options.major_width, fade),
                options.minor_color.lerp_to_gamma(options.major_color, fade),
            ),
            major: Stroke::new(options.major_width, options.major_color),
        }
    }

    fn get(&self, rank: GridRank) -> Stroke {
        match rank {
            GridRank::Minor => self.minor,
            GridRank::Middle => self.middle,
            GridRank::Major => self.major,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GridLevel {
    screen_step: f64,
    world_step: f64,
    fade: f32,
}

impl GridLevel {
    fn new(options: &GridOptions, zoom: f32, viewport: Rect) -> Self {
        let width = f64::from(viewport.width());
        let height = f64::from(viewport.height());
        let mut minimum =
            f64::from(options.min_spacing).max(width.max(height) / MAX_AXIS_INTERVALS);
        if options.kind == GridKind::Dots {
            minimum = minimum.max((width * height / MAX_DOTS).sqrt());
        }
        let zoom = f64::from(zoom);
        let base = f64::from(options.base_step) * zoom;
        let factor = SUBDIVISIONS as f64;
        let exponent = (minimum / base).log(factor).ceil() as i32;
        let mut world_step = f64::from(options.base_step) * factor.powi(exponent);
        let screen_step = world_step * zoom;
        // Компенсация округления log/pow на границах уровней.
        let corrected = if screen_step < minimum {
            exponent + 1
        } else if screen_step >= minimum * factor {
            exponent - 1
        } else {
            exponent
        };
        if corrected != exponent {
            world_step = f64::from(options.base_step) * factor.powi(corrected);
        }
        let screen_step = world_step * zoom;
        Self {
            screen_step,
            world_step,
            fade: ((screen_step / minimum - 1.0) / (factor - 1.0)).clamp(0.0, 1.0) as f32,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GridAxis {
    first: f64,
    step: f64,
    first_index: usize,
    count: usize,
}

impl GridAxis {
    #[expect(
        clippy::cast_sign_loss,
        reason = "Число делений и остаток номера неотрицательны и ограничены экранной плотностью"
    )]
    fn new(min: f32, max: f32, screen_center: f32, world_center: f32, level: GridLevel) -> Self {
        let center = f64::from(screen_center);
        // Остаток мирового центра сохраняет фазу без перевода огромного номера деления в целое число.
        let phase = f64::from(world_center).rem_euclid(level.world_step * GRID_CYCLE as f64)
            / level.world_step;
        let index = ((f64::from(min) - center) / level.screen_step + phase).ceil();
        let first = center + (index - phase) * level.screen_step;
        let count = ((f64::from(max) - first) / level.screen_step + 1.0).max(0.0) as usize;
        Self {
            first,
            step: level.screen_step,
            first_index: index.rem_euclid(GRID_CYCLE as f64) as usize,
            count,
        }
    }

    fn marks(self) -> impl Iterator<Item = (f32, GridRank)> {
        (0..self.count).map(move |index| {
            let rank = match (self.first_index + index) % GRID_CYCLE {
                0 => GridRank::Major,
                value if value % SUBDIVISIONS == 0 => GridRank::Middle,
                _ => GridRank::Minor,
            };
            ((self.first + index as f64 * self.step) as f32, rank)
        })
    }
}

/// Рисует только видимую часть сетки; подготовка не выделяет промежуточных буферов.
/// На больших областях шаг дополнительно укрупняется: до 512 интервалов на ось и примерно 16 тысяч точек.
pub fn paint_grid(context: &RenderContext<'_>, options: &GridOptions) {
    let clip = context.viewport.intersect(context.painter.clip_rect());
    let camera = context.camera;
    if !options.visible
        || !clip.is_finite()
        || !clip.is_positive()
        || !clip.size().is_finite()
        || !context.viewport.size().is_finite()
        || !camera.center.is_finite()
        || !camera.zoom.is_finite()
        || camera.zoom <= 0.0
    {
        return;
    }
    let level = GridLevel::new(options, camera.zoom, context.viewport);
    let center = context.viewport.center();
    let x = GridAxis::new(clip.left(), clip.right(), center.x, camera.center.x, level);
    let y = GridAxis::new(clip.top(), clip.bottom(), center.y, camera.center.y, level);
    let strokes = GridStrokes::new(options, level.fade);
    match options.kind {
        GridKind::Lines => {
            let vertical = x.marks().map(|(position, rank)| {
                (
                    [pos2(position, clip.top()), pos2(position, clip.bottom())],
                    rank,
                )
            });
            let horizontal = y.marks().map(|(position, rank)| {
                (
                    [pos2(clip.left(), position), pos2(clip.right(), position)],
                    rank,
                )
            });
            for (points, rank) in vertical.chain(horizontal) {
                let stroke = strokes.get(rank);
                if stroke.color != Color32::TRANSPARENT {
                    context.painter.line_segment(points, stroke);
                }
            }
        }
        GridKind::Dots => {
            for (x, x_rank) in x.marks() {
                for (y, y_rank) in y.marks() {
                    let stroke = strokes.get(x_rank.min(y_rank));
                    if stroke.color != Color32::TRANSPARENT {
                        context
                            .painter
                            .circle_filled(pos2(x, y), stroke.width, stroke.color);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
