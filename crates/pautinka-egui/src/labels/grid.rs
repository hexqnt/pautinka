use egui::{Rect, Vec2};

const NONE: usize = usize::MAX;

struct Obstacle {
    rect: Rect,
    weight: f64,
    seen: u64,
}

struct Link {
    obstacle: usize,
    next: usize,
}

/// Плоские буферы переиспользуются; длинные подписи не требуют отдельных Vec для каждой ячейки.
pub(super) struct Grid {
    viewport: Rect,
    columns: usize,
    rows: usize,
    cell: Vec2,
    heads: Vec<usize>,
    links: Vec<Link>,
    obstacles: Vec<Obstacle>,
    query: u64,
}

impl Grid {
    #[expect(
        clippy::cast_sign_loss,
        reason = "Число ячеек ограничено диапазоном 1..=256"
    )]
    pub fn clear(&mut self, viewport: Rect) {
        self.viewport = viewport;
        self.columns = ((viewport.width() / 64.0).ceil() as usize).clamp(1, 256);
        self.rows = ((viewport.height() / 64.0).ceil() as usize).clamp(1, 256);
        self.cell = viewport.size().max(Vec2::splat(1.0))
            / Vec2::new(self.columns as f32, self.rows as f32);
        self.heads.resize(self.columns * self.rows, NONE);
        self.heads.fill(NONE);
        self.links.clear();
        self.obstacles.clear();
        self.query = 0;
    }

    #[expect(
        clippy::cast_sign_loss,
        reason = "Отсечение по viewport даёт неотрицательные координаты ячеек"
    )]
    fn cells(&self, rect: Rect) -> Option<([usize; 2], [usize; 2])> {
        let clipped = rect.intersect(self.viewport);
        if !clipped.is_positive() {
            return None;
        }
        let min = (clipped.min - self.viewport.min) / self.cell;
        let max = (clipped.max - self.viewport.min) / self.cell;
        Some((
            [
                (min.x as usize).min(self.columns - 1),
                (max.x as usize).min(self.columns - 1),
            ],
            [
                (min.y as usize).min(self.rows - 1),
                (max.y as usize).min(self.rows - 1),
            ],
        ))
    }

    pub fn insert(&mut self, rect: Rect, weight: f32) {
        let Some(([left, right], [top, bottom])) = self.cells(rect) else {
            return;
        };
        let obstacle = self.obstacles.len();
        self.obstacles.push(Obstacle {
            rect,
            weight: f64::from(weight),
            seen: 0,
        });
        for row in top..=bottom {
            let start = row * self.columns;
            for head in &mut self.heads[start + left..=start + right] {
                self.links.push(Link {
                    obstacle,
                    next: *head,
                });
                *head = self.links.len() - 1;
            }
        }
    }

    pub fn overlap(&mut self, rect: Rect) -> f64 {
        let Some(([left, right], [top, bottom])) = self.cells(rect) else {
            return 0.0;
        };
        self.query = self.query.wrapping_add(1);
        if self.query == 0 {
            for obstacle in &mut self.obstacles {
                obstacle.seen = 0;
            }
            self.query = 1;
        }
        let mut cost = 0.0;
        let candidate_area = area(rect).max(f64::MIN_POSITIVE);
        for row in top..=bottom {
            let start = row * self.columns;
            for &head in &self.heads[start + left..=start + right] {
                let mut link = head;
                while link != NONE {
                    let entry = &self.links[link];
                    let obstacle = &mut self.obstacles[entry.obstacle];
                    if obstacle.seen != self.query {
                        obstacle.seen = self.query;
                        cost +=
                            area(rect.intersect(obstacle.rect)) / candidate_area * obstacle.weight;
                    }
                    link = entry.next;
                }
            }
        }
        cost
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            viewport: Rect::ZERO,
            columns: 0,
            rows: 0,
            cell: Vec2::ZERO,
            heads: Vec::new(),
            links: Vec::new(),
            obstacles: Vec::new(),
            query: 0,
        }
    }
}

pub(super) fn area(rect: Rect) -> f64 {
    let width = (f64::from(rect.max.x) - f64::from(rect.min.x)).max(0.0);
    let height = (f64::from(rect.max.y) - f64::from(rect.min.y)).max(0.0);
    width * height
}
