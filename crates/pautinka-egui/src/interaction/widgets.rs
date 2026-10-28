use egui::{Id, Modifiers, Pos2, Rect, Response, Sense, Ui};

pub(crate) struct WidgetInput {
    pub pointer: Option<Pos2>,
    pub origin: Option<Pos2>,
    pub modifiers: Modifiers,
    focused: Option<Id>,
    pub accessibility: bool,
    pub blocked: bool,
    pub viewport: Rect,
}

impl WidgetInput {
    pub fn new(ui: &Ui, viewport: Rect, blocked: bool) -> Self {
        let (pointer, origin, modifiers) = ui.input(|input| {
            (
                input.pointer.interact_pos(),
                input.pointer.press_origin(),
                input.modifiers,
            )
        });
        Self {
            pointer,
            origin,
            modifiers,
            viewport,
            blocked,
            focused: ui.memory(egui::Memory::focused),
            accessibility: ui
                .ctx()
                .accesskit_node_builder(ui.scope_id(), |_| ())
                .is_some(),
        }
    }

    pub fn click_sense(&self, id: Id, hovered: bool, keyboard: bool) -> Sense {
        if hovered || self.accessibility || self.focused == Some(id) {
            Sense::click()
        } else if keyboard {
            Sense::focusable_noninteractive()
        } else {
            Sense::hover()
        }
    }
}

pub(crate) fn synthetic_click(response: &Response) -> bool {
    response.clicked() && !response.clicked_by(egui::PointerButton::Primary)
}
