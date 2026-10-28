use egui::{Pos2, Vec2};

use crate::{ElementId, GroupId, NodeId};

/// Намерения пользователя; владелец модели сам применяет изменения и ведёт историю.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanvasEvent {
    /// Запрос желаемого состояния; холст сам не скрывает элементы и не меняет маршруты.
    GroupCollapseRequested {
        group: GroupId,
        collapsed: bool,
    },
    GroupDragStarted {
        group: GroupId,
    },
    /// Полное мировое смещение относительно начала жеста; состав перемещаемых элементов определяет приложение.
    GroupDragged {
        group: GroupId,
        delta: Vec2,
    },
    GroupDragFinished {
        group: GroupId,
        delta: Vec2,
    },
    GroupDragCancelled {
        group: GroupId,
    },
    ContextRequested(Option<ElementId>),
    ContextPositionRequested {
        target: Option<ElementId>,
        position: Pos2,
    },
    Clicked {
        target: Option<ElementId>,
        position: Pos2,
    },
    Activated {
        element: ElementId,
        position: Pos2,
    },
    SelectionChanged(Option<ElementId>),
    NodeDragCancelled {
        node: NodeId,
    },
    NodeDragStarted {
        node: NodeId,
        position: Pos2,
    },
    NodeDragged {
        node: NodeId,
        position: Pos2,
    },
    NodeDragFinished {
        node: NodeId,
        position: Pos2,
    },
}
