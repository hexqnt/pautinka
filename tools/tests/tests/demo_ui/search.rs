use super::*;

#[test]
fn closed_station_can_be_found_and_selected_but_cannot_be_a_route_endpoint() {
    let mut harness = demo_harness(Vec2::new(1440.0, 1700.0));
    harness.run();
    search_station(&mut harness, "Фрунзенская");
    click_label(&mut harness, "→ Фрунзенская");
    assert!(
        harness
            .query_by_label("Закрыта для входа и выхода; доступен проезд")
            .is_some()
    );
    for label in ["Отсюда", "Сюда"] {
        assert!(harness.get_by_label(label).accesskit_node().is_disabled());
    }
    assert!(
        harness
            .query_by_label("A  Отправление не выбрано")
            .is_some()
    );
    assert!(harness.query_by_label("B  Назначение не выбрано").is_some());
    click_label(&mut harness, "Поиск станции");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_label("Поиск станции").type_text("Зенит");
    harness.run();
    click_label(&mut harness, "→ Зенит");
    assert!(
        !harness
            .get_by_label("Отсюда")
            .accesskit_node()
            .is_disabled()
    );
    click_label(&mut harness, "Отсюда");
    assert!(harness.query_by_label("A  Зенит").is_some());
}

#[test]
fn stations_with_the_same_name_are_distinguished_by_line_in_search() {
    let mut harness = desktop();
    harness.run();
    search_station(&mut harness, "Площадь Александра Невского");
    for line in [3, 4] {
        assert!(
            harness
                .query_by_label(&format!("→ Площадь Александра Невского · линия {line}"))
                .is_some()
        );
    }
    click_label(&mut harness, "→ Площадь Александра Невского · линия 4");
    click_label(&mut harness, "Отсюда");
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Новочеркасская")
        .click_accesskit();
    harness.run();
    click_label(&mut harness, "Сюда");
    assert!(
        harness
            .query_by_label("Станций: 2 · Пересадок: 0")
            .is_some()
    );
}

#[test]
fn search_select_route_and_reset() {
    let mut harness = desktop();
    harness.run();
    search_station(&mut harness, "Беговая");
    click_label(&mut harness, "→ Беговая");
    click_label(&mut harness, "Отсюда");
    search_station(&mut harness, "несуществующая");
    assert!(harness.query_by_label("Станция не найдена").is_some());
    // Конечную станцию выбираем на холсте, без прямого изменения состояния приложения.
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Зенит")
        .click_accesskit();
    harness.run();
    click_label(&mut harness, "Сюда");
    assert!(harness.query_by_label("4 мин").is_some());
    click_label(&mut harness, "Сбросить маршрут");
    assert!(harness.query_by_label("4 мин").is_none());
}

#[test]
fn fuzzy_dropdown_selects_abbreviation_and_closes_after_click() {
    let mut harness = desktop();
    harness.run();
    search_station(&mut harness, "бгв");
    click_label(&mut harness, "→ Беговая");
    assert!(harness.query_by_label("→ Беговая").is_none());
    assert!(harness.query_by_label("Отсюда").is_some());
    assert_eq!(
        harness.get_by_label("Поиск станции").value().as_deref(),
        Some("Беговая")
    );
    click_label(&mut harness, "Поиск станции");
    assert!(harness.query_by_label("→ Беговая").is_some());
    click_label(&mut harness, "Все линии");
    assert!(harness.query_by_label("→ Беговая").is_none());
}

#[test]
fn fuzzy_dropdown_supports_arrow_keys_enter_and_escape() {
    let mut harness = desktop();
    harness.run();
    search_station(&mut harness, "пл");
    let options: Vec<_> = harness
        .get_all_by_label_contains("→ ")
        .map(|row| row.accesskit_node().label().unwrap())
        .collect();
    assert!(options.len() > 1);
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        harness.get_by_label("Поиск станции").value().as_deref(),
        options[1]
            .trim_start_matches("→ ")
            .split(" · линия ")
            .next()
    );
    assert!(harness.query_by_label("Отсюда").is_some());
    assert!(harness.query_by_label_contains("→ ").is_none());
    click_label(&mut harness, "Поиск станции");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(harness.query_by_label_contains("→ ").is_none());
    assert!(harness.query_by_label("Отсюда").is_some());
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert!(harness.get_all_by_label_contains("→ ").next().is_some());
}

#[test]
fn clearing_fuzzy_query_closes_the_dropdown() {
    let mut harness = desktop();
    harness.run();
    search_station(&mut harness, "бгв");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.key_press(egui::Key::Backspace);
    harness.run();
    assert_eq!(
        harness.get_by_label("Поиск станции").value().as_deref(),
        Some("")
    );
    assert!(harness.query_by_label_contains("→ ").is_none());
    assert!(harness.query_by_label("Станция не найдена").is_none());
}

#[test]
fn switching_city_rebuilds_search_lines_and_clears_route() {
    let mut harness = demo_harness(Vec2::new(1440.0, 1700.0));
    harness.run();
    search_station(&mut harness, "Беговая");
    click_label(&mut harness, "→ Беговая");
    click_label(&mut harness, "Отсюда");
    click_label(&mut harness, "Город");
    click_label(&mut harness, "Москва");
    assert_eq!(
        harness.get_by_label("Поиск станции").value().as_deref(),
        Some("")
    );
    assert!(harness.query_by_label("Отсюда").is_none());
    assert!(harness.query_by_label_contains("Сокольническая").is_some());
    assert!(
        harness
            .query_by_label_contains("Кировско-Выборгская")
            .is_none()
    );
    search_station(&mut harness, "Охотный Ряд");
    click_label(&mut harness, "→ Охотный Ряд");
    click_label(&mut harness, "Отсюда");
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Лубянка")
        .click_accesskit();
    harness.run();
    click_label(&mut harness, "Сюда");
    assert!(harness.query_by_label("3 мин").is_some());
    click_label(&mut harness, "Город");
    click_label(&mut harness, "Санкт-Петербург");
    assert!(harness.query_by_label("3 мин").is_none());
    assert!(
        harness
            .query_by_label_contains("Кировско-Выборгская")
            .is_some()
    );
    assert_eq!(
        harness.get_by_label("Поиск станции").value().as_deref(),
        Some("")
    );
}
