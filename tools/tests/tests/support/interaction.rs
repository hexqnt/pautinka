use egui::{Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};

pub(super) fn click_label<T>(harness: &mut Harness<'_, T>, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

pub(super) fn click_role_label<T>(harness: &mut Harness<'_, T>, role: Role, label: &str) {
    harness.get_by_role_and_label(role, label).click();
    harness.run();
}

pub(super) fn pointer_button(
    pos: Pos2,
    button: PointerButton,
    pressed: bool,
    modifiers: Modifiers,
) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    }
}
