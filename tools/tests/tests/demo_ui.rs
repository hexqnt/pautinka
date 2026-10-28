#[path = "support/interaction.rs"]
mod support;

#[path = "demo_ui/interaction.rs"]
mod interaction;
#[path = "demo_ui/layout.rs"]
mod layout;
#[path = "demo_ui/localization.rs"]
mod localization;
#[path = "demo_ui/rendering.rs"]
mod rendering;
#[path = "demo_ui/search.rs"]
mod search;

use egui::Vec2;
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use pautinka_demo::Demo;

use support::{click_label, click_role_label, pointer_button};

fn desktop() -> Harness<'static, Demo> {
    demo_harness(Vec2::new(1440.0, 1100.0))
}

fn demo_harness(size: Vec2) -> Harness<'static, Demo> {
    localized_demo_harness(size, pautinka_demo::Language::Russian)
}

fn localized_demo_harness(size: Vec2, language: pautinka_demo::Language) -> Harness<'static, Demo> {
    Harness::builder()
        .with_size(size)
        .build_ui_state(|ui, demo| demo.show(ui), Demo::with_language(language))
}

fn search_station(harness: &mut Harness<'_, Demo>, query: &str) {
    click_label(harness, "Поиск станции");
    harness.get_by_label("Поиск станции").type_text(query);
    harness.run();
}

fn close_menu(harness: &mut Harness<'_, Demo>) {
    if egui::Popup::is_any_open(&harness.ctx) {
        harness.key_press(egui::Key::Escape);
        harness.run();
    }
}

fn open_display_menu(harness: &mut Harness<'_, Demo>, label: &str) {
    close_menu(harness);
    click_label(harness, label);
}
