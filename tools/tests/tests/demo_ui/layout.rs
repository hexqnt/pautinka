use super::*;

#[test]
fn desktop_explorer_collapses_completely_and_restores_route_state() {
    for width in [800.0, 1024.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 1100.0));
        harness.run();
        search_station(&mut harness, "Беговая");
        click_label(&mut harness, "→ Беговая");
        click_label(&mut harness, "Отсюда");
        let map = harness.get_by_label("Схема метро Санкт-Петербурга").rect();

        click_label(&mut harness, "Скрыть боковую панель");
        assert!(harness.query_by_label("Все линии").is_none());
        assert!(harness.query_by_label("A  Беговая").is_none());
        let expanded_map = harness.get_by_label("Схема метро Санкт-Петербурга").rect();
        assert!(expanded_map.left() < 40.0);
        assert!(map.left() - expanded_map.left() >= 300.0);
        assert!(expanded_map.width() > map.width());
        assert!(expanded_map.right() <= width);
        let toggle = harness.get_by_label("Показать боковую панель").rect();
        assert!(toggle.left() >= 0.0 && toggle.right() <= width);
        assert!(toggle.bottom() < expanded_map.top());
        assert!(harness.query_by_label("Поиск станции").is_some());

        click_label(&mut harness, "Показать боковую панель");
        assert!(harness.query_by_label("Все линии").is_some());
        assert!(harness.query_by_label("A  Беговая").is_some());
        assert!(harness.query_by_label("Отсюда").is_some());
        assert_eq!(
            harness
                .get_by_label("Схема метро Санкт-Петербурга")
                .rect()
                .left(),
            map.left()
        );
    }
}

#[test]
fn map_controls_and_line_selection() {
    let mut harness = desktop();
    harness.run();
    click_label(&mut harness, "1   Кировско-Выборгская");
    open_display_menu(&mut harness, "Линии и подписи");
    click_label(&mut harness, "Подписи станций");
    close_menu(&mut harness);
    for label in ["+", "−", "Вся схема"] {
        assert!(harness.query_by_label(label).is_none());
    }
    assert!(
        harness
            .query_by_label("Схема метро Санкт-Петербурга")
            .is_some()
    );
}

#[test]
fn mobile_header_search_selects_station_without_opening_explorer() {
    let mut harness = demo_harness(Vec2::new(390.0, 844.0));
    harness.run();
    let search = harness.get_by_label("Поиск станции").rect();
    assert!(search.left() >= 0.0 && search.right() <= 390.0);
    assert!(search.bottom() < harness.get_by_label("Линии и подписи").rect().top());
    assert!(harness.query_by_label("Все линии").is_none());
    search_station(&mut harness, "Беговая");
    assert!(harness.query_by_label("→ Беговая").is_some());
    click_label(&mut harness, "→ Беговая");
    click_label(&mut harness, "Станции");
    assert!(harness.query_by_label("Отсюда").is_some());
    assert_eq!(harness.get_all_by_label("Поиск станции").count(), 1);
}

#[test]
fn city_and_search_controls_are_in_the_header_on_desktop_and_mobile() {
    for width in [360.0, 390.0, 780.0, 800.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 1100.0));
        harness.run();
        let map_top = harness.get_by_label("Линии и подписи").rect().top();
        for label in [
            "Городские связи",
            "/  интерактивное демо",
            "Город",
            "Поиск станции",
        ] {
            assert!(
                harness
                    .query_by_role_and_label(egui::accesskit::Role::Label, label)
                    .is_none()
            );
        }
        assert!(
            harness
                .query_by_role_and_label(egui::accesskit::Role::Label, "pautinka")
                .is_none()
        );
        let link = harness
            .get_by_role_and_label(egui::accesskit::Role::Link, "pautinka")
            .rect();
        assert!(link.left() >= 0.0 && link.right() <= width && link.bottom() < map_top);
        let icon = harness
            .get_by_role_and_label(egui::accesskit::Role::Image, "Поиск")
            .rect();
        let field = harness.get_by_label("Поиск станции").rect();
        assert!(icon.right() < field.left());
        assert!(
            (icon.center().y - field.center().y).abs() < 2.0,
            "width={width}, icon={icon:?}, field={field:?}"
        );
        if width >= 800.0 {
            let link = harness
                .get_by_role_and_label(egui::accesskit::Role::Link, "pautinka")
                .rect();
            for rect in [field, harness.get_by_label("Город").rect()] {
                assert!(rect.top() < link.bottom() && link.top() < rect.bottom());
                assert!(rect.right() < link.left());
            }
        }
        for label in ["Город", "Поиск станции"] {
            let rect = harness.get_by_label(label).rect();
            assert!(
                rect.left() >= 0.0 && rect.right() <= width,
                "{label} at {width}"
            );
            assert!(rect.bottom() < map_top, "{label} at {width}");
        }
        click_label(&mut harness, "Город");
        click_label(&mut harness, "Москва");
        assert!(harness.query_by_label("Схема метро Москвы").is_some());
        search_station(&mut harness, "Охотный Ряд");
        click_label(&mut harness, "→ Охотный Ряд");
        assert_eq!(
            harness.get_by_label("Поиск станции").value().as_deref(),
            Some("Охотный Ряд")
        );
    }
}

#[test]
fn editing_controls_are_in_the_footer_and_snapping_menu_opens_on_small_screens() {
    for width in [360.0, 390.0, 1440.0] {
        let height = 844.0;
        let mut harness = demo_harness(Vec2::new(width, height));
        harness.run();
        let center = harness.get_by_label("Режимы привязки").rect().center().y;
        let snapping = harness.get_by_label("Привязка").rect();
        assert!(
            (snapping.center().y - center).abs() < 1.0,
            "{snapping:?} at {width}"
        );
        assert!(harness.query_by_label("Перетаскивание станций").is_none());
        assert!(harness.query_by_label("Rust + egui + pautinka").is_none());
        assert!(harness.query_by_label_contains("Фон — обзор").is_none());
        let map_bottom = harness
            .get_by_label("Схема метро Санкт-Петербурга")
            .rect()
            .bottom();
        for label in ["Привязка", "Режимы привязки"] {
            let rect = harness.get_by_label(label).rect();
            assert!(rect.top() > map_bottom, "{label} at {width}");
            assert!(
                rect.left() >= 0.0 && rect.right() <= width,
                "{label} at {width}"
            );
            assert!(rect.bottom() <= height, "{label} at {width}");
            assert!(rect.height() < 40.0, "{label} must not wrap at {width}");
        }
        click_label(&mut harness, "Режимы привязки");
        for label in ["Горизонталь", "Вертикаль", "Диагональ / (45°)"]
        {
            let rect = harness.get_by_label(label).rect();
            assert!(
                rect.top() >= 0.0 && rect.bottom() <= height,
                "{label} at {width}"
            );
        }
        click_label(&mut harness, "Горизонталь");
    }
}

#[test]
fn github_link_opens_the_library_repository() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1440.0, 1100.0))
        .build_ui_state(
            |ui, (demo, opened): &mut (Demo, Option<String>)| {
                demo.show(ui);
                ui.ctx().output(|output| {
                    for command in &output.commands {
                        if let egui::OutputCommand::OpenUrl(url) = command {
                            *opened = Some(url.url.clone());
                        }
                    }
                });
            },
            (Demo::with_language(pautinka_demo::Language::Russian), None),
        );
    harness.run();
    click_role_label(&mut harness, egui::accesskit::Role::Link, "pautinka");
    assert_eq!(
        harness.state().1.as_deref(),
        Some("https://github.com/hexqnt/pautinka")
    );
}

#[test]
fn display_settings_are_grouped_and_accessible_on_small_viewports() {
    for (width, height) in [
        (360.0, 480.0),
        (390.0, 844.0),
        (800.0, 844.0),
        (1440.0, 1100.0),
    ] {
        let mut harness = demo_harness(Vec2::new(width, height));
        harness.run();
        let map = harness.get_by_label("Схема метро Санкт-Петербурга").rect();
        for (menu, control) in [
            ("Линии и подписи", "Подписи станций"),
            ("Пересечения", "Разрыв"),
            ("Навигация", "Мини-карта"),
            ("Сетка", "Показывать сетку"),
        ] {
            let rect = harness.get_by_label(menu).rect();
            assert!(
                rect.left() >= 0.0 && rect.right() <= width,
                "{menu} at {width}"
            );
            assert!(rect.bottom() < map.top());
            assert!(harness.query_by_label(control).is_none());
            open_display_menu(&mut harness, menu);
            let node = harness.get_by_label(control);
            node.scroll_to_me();
            harness.run();
            let rect = harness.get_by_label(control).rect();
            assert!(
                rect.left() >= 0.0 && rect.right() <= width,
                "{control} at {width}"
            );
            assert!(
                rect.top() >= 0.0 && rect.bottom() <= height,
                "{control} at {height}"
            );
            close_menu(&mut harness);
            assert!(harness.query_by_label(control).is_none());
        }
    }
}
