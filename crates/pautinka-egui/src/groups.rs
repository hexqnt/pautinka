mod interaction;
mod paint;
mod ui;

use egui::{Color32, FontId, Pos2, Rect, Stroke, Vec2};

use crate::{Camera, ElementState, GroupFrame, GroupId, HitPoint};

pub use paint::paint_default_group;

pub(super) const PICK_MARGIN: f32 = 4.0;

/// Состояние отображения задаёт приложение; скрытие потомков и маршрутизация не выполняются холстом.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupState {
    Expanded,
    Collapsed,
}

/// Возможности выбора и переноса группы совпадают с политикой отдельного узла.
pub type GroupInteraction = crate::NodeInteraction;

/// Заимствованная рамка без иерархии и владения содержимым; порядок вложенных рамок задаёт `Graph::group`.
#[derive(Clone, Copy, Debug)]
pub struct Group<'a> {
    pub id: GroupId,
    pub frame: GroupFrame,
    pub label: &'a str,
    pub accessibility_label: &'a str,
    pub color: Color32,
    /// `None` скрывает кнопку; `Some` показывает состояние и позволяет запросить его смену.
    pub state: Option<GroupState>,
}

impl<'a> Group<'a> {
    pub fn new(id: GroupId, frame: GroupFrame, label: &'a str) -> Self {
        Self {
            id,
            frame,
            label,
            accessibility_label: label,
            color: Color32::from_rgb(100, 145, 190),
            state: None,
        }
    }

    /// Заголовок и рамка выбираются, пустая область раскрытой группы остаётся фоном холста.
    pub(crate) fn hit_test(self, point: HitPoint, zoom: f32) -> bool {
        let bounds = self.frame.bounds();
        let relative = point.relative_to(bounds.min);
        let rect = Rect::from_min_size(Pos2::ZERO, bounds.size());
        let margin = PICK_MARGIN / zoom;
        rect.expand(margin).contains(relative.to_pos2())
            && (self.state == Some(GroupState::Collapsed)
                || relative.y <= self.frame.header().height()
                || !rect.shrink(margin).contains(relative.to_pos2()))
    }

    fn screen_geometry(self, camera: Camera, viewport: Rect) -> GroupScreenGeometry {
        let to_screen = |rect: Rect| {
            Rect::from_min_max(
                camera.to_screen(rect.min, viewport),
                camera.to_screen(rect.max, viewport),
            )
        };
        let rect = to_screen(self.frame.bounds());
        let header = to_screen(self.frame.header());
        let toggle = self.state.map(|_| {
            Rect::from_min_size(header.min, Vec2::splat(header.size().min_elem().min(24.0)))
        });
        GroupScreenGeometry {
            rect,
            header,
            toggle,
            collapsed: self.state == Some(GroupState::Collapsed),
        }
    }
}

#[derive(Clone, Copy)]
struct GroupScreenGeometry {
    rect: Rect,
    header: Rect,
    toggle: Option<Rect>,
    collapsed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupPart {
    Header,
    Toggle,
    Frame,
}

impl GroupScreenGeometry {
    fn part_at(self, point: Pos2) -> Option<GroupPart> {
        if self.toggle.is_some_and(|rect| rect.contains(point)) {
            Some(GroupPart::Toggle)
        } else if self.header.contains(point) {
            Some(GroupPart::Header)
        } else {
            (self.rect.expand(PICK_MARGIN).contains(point)
                && (self.collapsed || !self.rect.shrink(PICK_MARGIN).contains(point)))
            .then_some(GroupPart::Frame)
        }
    }

    /// Интерьер раскрытой рамки остаётся фоном холста, доступным для панорамирования и выделения.
    fn interaction_rect(self, hit: Option<GroupPart>) -> Rect {
        if self.collapsed || hit == Some(GroupPart::Frame) {
            self.rect.expand(PICK_MARGIN)
        } else {
            self.header
        }
    }
}

#[derive(Clone, Debug)]
pub struct GroupStyle {
    pub fill: Color32,
    pub header_fill: Color32,
    pub stroke: Stroke,
    pub halo: Option<Stroke>,
    pub label_color: Color32,
    pub label_font: FontId,
}

pub struct GroupVisual {
    pub index: usize,
    pub rect: Rect,
    pub header_rect: Rect,
    pub toggle_rect: Option<Rect>,
    pub state: ElementState,
    pub style: GroupStyle,
}

pub(super) struct GroupDrag {
    id: GroupId,
    index: usize,
    origin: HitPoint,
    delta: Vec2,
}
