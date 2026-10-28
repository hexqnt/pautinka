use super::*;
use pautinka_demo::Language;

fn switch_language(harness: &mut Harness<'_, Demo>, language: Language) {
    close_menu(harness);
    click_label(harness, "Language / Язык");
    click_label(harness, language.name());
    assert_eq!(harness.state().language(), language);
}

#[test]
fn language_selector_stays_on_the_right_and_works_with_snapping_disabled() {
    for width in [360.0, 390.0, 800.0, 1440.0] {
        for language in Language::ALL {
            let mut harness = localized_demo_harness(Vec2::new(width, 844.0), language);
            harness.run();
            let (snapping, modes, map) = match language {
                Language::English => ("Snapping", "Snap modes", "Saint Petersburg metro map"),
                Language::Russian => (
                    "Привязка",
                    "Режимы привязки",
                    "Схема метро Санкт-Петербурга",
                ),
            };
            let selector = harness.get_by_label("Language / Язык").rect();
            assert!(
                (selector.right() - (width - 18.0)).abs() < 1.0,
                "{selector:?} at {width}"
            );
            let map = harness.get_by_label(map).rect();
            assert!(selector.top() > map.bottom());
            assert!(selector.bottom() <= 844.0);
            for label in [snapping, modes] {
                let rect = harness.get_by_label(label).rect();
                assert!(
                    rect.left() >= 0.0 && rect.right() < selector.left(),
                    "{label}: {rect:?} at {width}"
                );
                assert!(rect.bottom() <= 844.0);
                assert!(rect.height() < 40.0);
            }
            click_label(&mut harness, snapping);
            assert!(harness.get_by_label(modes).accesskit_node().is_disabled());
            assert!(
                !harness
                    .get_by_label("Language / Язык")
                    .accesskit_node()
                    .is_disabled()
            );
            let other = match language {
                Language::English => Language::Russian,
                Language::Russian => Language::English,
            };
            switch_language(&mut harness, other);
            let translated = match other {
                Language::English => "Snapping",
                Language::Russian => "Привязка",
            };
            assert_eq!(
                harness.get_by_label(translated).accesskit_node().toggled(),
                Some(egui::accesskit::Toggled::False)
            );
        }
    }
}

#[test]
fn language_switch_preserves_route_search_and_display_settings() {
    let mut harness = desktop();
    harness.run();
    search_station(&mut harness, "Беговая");
    click_label(&mut harness, "→ Беговая");
    click_label(&mut harness, "Отсюда");
    click_label(&mut harness, "Поиск станции");
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_label("Поиск станции").type_text("Зенит");
    harness.run();
    click_label(&mut harness, "→ Зенит");
    click_label(&mut harness, "Сюда");
    open_display_menu(&mut harness, "Линии и подписи");
    click_label(&mut harness, "Кривые");
    click_label(&mut harness, "Подписи станций");
    switch_language(&mut harness, Language::English);
    assert!(
        harness
            .query_by_label("Saint Petersburg metro map")
            .is_some()
    );
    assert!(harness.query_by_label("A  Беговая").is_some());
    assert!(harness.query_by_label("B  Зенит").is_some());
    assert!(harness.query_by_label("Reset route").is_some());
    assert!(
        harness
            .query_by_label("Stations: 2 · Transfers: 0")
            .is_some()
    );
    assert_eq!(
        harness.get_by_label("Station search").value().as_deref(),
        Some("Зенит")
    );
    assert!(
        harness
            .query_by_label("3   Nevsko-Vasileostrovskaya")
            .is_some()
    );
    open_display_menu(&mut harness, "Lines and labels");
    assert_eq!(
        harness.get_by_label("Curved").accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    assert_eq!(
        harness
            .get_by_label("Station labels")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    open_display_menu(&mut harness, "Crossings");
    assert!(!harness.get_by_label("Gap").accesskit_node().is_disabled());
    assert!(
        !harness
            .get_by_label("Bridge")
            .accesskit_node()
            .is_disabled()
    );
    switch_language(&mut harness, Language::Russian);
    assert!(harness.query_by_label("A  Беговая").is_some());
    assert!(harness.query_by_label("B  Зенит").is_some());
    assert!(harness.query_by_label("Сбросить маршрут").is_some());
    open_display_menu(&mut harness, "Линии и подписи");
    assert_eq!(
        harness.get_by_label("Кривые").accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True)
    );
}

#[test]
fn english_survives_city_changes_and_search_disambiguation_is_translated() {
    let mut harness = localized_demo_harness(Vec2::new(1440.0, 1100.0), Language::English);
    harness.run();
    click_label(&mut harness, "Station search");
    harness
        .get_by_label("Station search")
        .type_text("Фрунзенская");
    harness.run();
    click_label(&mut harness, "→ Фрунзенская");
    assert!(
        harness
            .query_by_label("Closed for boarding and exiting; through trains only")
            .is_some()
    );
    assert!(
        harness
            .get_by_label("From here")
            .accesskit_node()
            .is_disabled()
    );
    click_label(&mut harness, "City");
    click_label(&mut harness, "Moscow");
    assert_eq!(harness.state().language(), Language::English);
    assert!(harness.query_by_label("Moscow metro map").is_some());
    click_label(&mut harness, "Station search");
    harness
        .get_by_label("Station search")
        .type_text("Арбатская");
    harness.run();
    assert!(harness.query_by_label("→ Арбатская · line 3").is_some());
    assert!(harness.query_by_label("→ Арбатская · line 4").is_some());
    switch_language(&mut harness, Language::Russian);
    assert!(harness.query_by_label("Схема метро Москвы").is_some());
    click_label(&mut harness, "Поиск станции");
    assert!(harness.query_by_label("→ Арбатская · линия 3").is_some());
    assert!(harness.query_by_label("→ Арбатская · линия 4").is_some());
}

#[test]
fn english_settings_are_usable_on_small_screens() {
    for size in [Vec2::new(360.0, 480.0), Vec2::new(390.0, 844.0)] {
        let mut harness = localized_demo_harness(size, Language::English);
        harness.run();
        for (menu, control) in [
            ("Lines and labels", "Station labels"),
            ("Crossings", "Gap"),
            ("Navigation", "Mini-map"),
            ("Grid", "Show grid"),
        ] {
            open_display_menu(&mut harness, menu);
            harness.get_by_label(control).scroll_to_me();
            harness.run();
            let rect = harness.get_by_label(control).rect();
            assert!(
                rect.left() >= 0.0 && rect.right() <= size.x,
                "{control}: {rect:?}"
            );
            assert!(
                rect.top() >= 0.0 && rect.bottom() <= size.y,
                "{control}: {rect:?}"
            );
            close_menu(&mut harness);
        }
        open_display_menu(&mut harness, "Grid");
        click_label(&mut harness, "Show grid");
        click_label(&mut harness, "Grid dots");
        harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Major mark size")
            .scroll_to_me();
        harness.run();
        let rect = harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Major mark size")
            .rect();
        assert!(rect.left() >= 0.0 && rect.right() <= size.x);
        assert!(rect.top() >= 0.0 && rect.bottom() <= size.y);
    }
}
