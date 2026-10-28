use egui::{CursorIcon, Modifiers, Pos2, Rect, Ui, Vec2};

use crate::{Canvas, CanvasEvent, ElementId, Graph, Selection};

use super::{GroupDrag, GroupPart};

impl Canvas {
    pub(crate) fn group_cursor(
        &self,
        graph: &(impl Graph + ?Sized),
        index: usize,
        point: Pos2,
        viewport: Rect,
        modifiers: Modifiers,
    ) -> Option<CursorIcon> {
        let options = self.options.interaction;
        match graph
            .group(index)
            .screen_geometry(self.camera, viewport)
            .part_at(point)
        {
            Some(GroupPart::Toggle) if options.group_toggle => Some(CursorIcon::PointingHand),
            Some(GroupPart::Header)
                if options.group_drag
                    && graph.group_interaction(index).draggable
                    && options.group_drag_modifier.matches(modifiers) =>
            {
                Some(CursorIcon::Grab)
            }
            _ => None,
        }
    }

    pub(crate) fn start_group_drag(
        &mut self,
        graph: &(impl Graph + ?Sized),
        index: usize,
        origin: Pos2,
        viewport: Rect,
    ) {
        let Some(origin) = self.camera.to_world_precise(origin, viewport) else {
            return;
        };
        let id = graph.group(index).id;
        self.focus_requested = true;
        self.initial_fit_complete = true;
        self.group_drag = Some(GroupDrag {
            id,
            index,
            origin,
            delta: Vec2::ZERO,
        });
        if self.options.interaction.selection && !self.is_selected(ElementId::Group(id)) {
            self.interactive_select(Some(Selection::Group(index)), egui::Modifiers::NONE, graph);
        }
        self.events
            .push(CanvasEvent::GroupDragStarted { group: id });
    }

    pub(crate) fn update_group_drag(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        viewport: Rect,
    ) {
        let Some(drag) = &mut self.group_drag else {
            return;
        };
        let resolved =
            if graph.identity(Selection::Group(drag.index)) == Some(ElementId::Group(drag.id)) {
                Some(Selection::Group(drag.index))
            } else {
                graph.resolve(ElementId::Group(drag.id))
            };
        let enabled = match resolved {
            Some(Selection::Group(index)) => {
                drag.index = index;
                ui.is_enabled()
                    && self.options.interaction.group_drag
                    && graph.group_interaction(index).draggable
            }
            _ => false,
        };
        let input = crate::interaction::drag_pointer(ui)
            .filter(|_| enabled)
            .and_then(|(pointer, released)| {
                let [x, y] = self
                    .camera
                    .to_world_precise(pointer, viewport)?
                    .coordinates();
                let [origin_x, origin_y] = drag.origin.coordinates();
                let delta = Vec2::new((x - origin_x) as f32, (y - origin_y) as f32);
                delta.is_finite().then_some((delta, released))
            });
        let Some((delta, released)) = input else {
            self.events
                .push(CanvasEvent::GroupDragCancelled { group: drag.id });
            self.group_drag = None;
            ui.ctx().request_repaint();
            return;
        };
        if released {
            self.events.push(CanvasEvent::GroupDragFinished {
                group: drag.id,
                delta,
            });
            self.group_drag = None;
        } else if delta != drag.delta {
            drag.delta = delta;
            self.events.push(CanvasEvent::GroupDragged {
                group: drag.id,
                delta,
            });
        } else {
            return;
        }
        ui.ctx().request_repaint();
    }
}
