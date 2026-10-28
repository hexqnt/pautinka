use super::*;

#[test]
fn grid_controls_switch_the_pattern_and_preserve_settings_across_toggles_and_cities() {
    for width in [360.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 844.0));
        harness.run();
        let enabled = |harness: &Harness<'_, Demo>| {
            harness
                .get_by_label("Показывать сетку")
                .accesskit_node()
                .toggled()
        };
        open_display_menu(&mut harness, "Сетка");
        assert_eq!(enabled(&harness), Some(egui::accesskit::Toggled::False));
        assert!(
            harness
                .get_by_label("Точки сетки")
                .accesskit_node()
                .is_disabled()
        );
        close_menu(&mut harness);
        click_label(&mut harness, "Режимы привязки");
        click_label(&mut harness, "К базовой сетке");
        harness.key_press(egui::Key::Escape);
        harness.run();
        open_display_menu(&mut harness, "Сетка");
        click_label(&mut harness, "Показывать сетку");
        assert_eq!(enabled(&harness), Some(egui::accesskit::Toggled::True));
        click_label(&mut harness, "Точки сетки");
        harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Базовый шаг")
            .focus();
        harness.run();
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        let base_step = harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Базовый шаг")
            .accesskit_node()
            .numeric_value()
            .unwrap();
        assert!(base_step > 10.0);
        let slider =
            harness.get_by_role_and_label(egui::accesskit::Role::Slider, "Интервал на экране");
        let before = slider.accesskit_node().numeric_value().unwrap();
        slider.focus();
        harness.run();
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        let spacing = harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Интервал на экране")
            .accesskit_node()
            .numeric_value()
            .unwrap();
        assert!(spacing > before);
        harness.key_press(egui::Key::Escape);
        harness.run();
        let dots = |harness: &Harness<'_, Demo>| {
            harness.output().shapes.iter().filter(|shape| matches!(&shape.shape,
            egui::Shape::Circle(circle) if circle.fill.a() > 0 && circle.fill.a() < 255 && circle.fill.r() == 0 && circle.fill.g() == 0 && circle.fill.b() == 0
        )).count()
        };
        assert!(dots(&harness) > 0);
        open_display_menu(&mut harness, "Сетка");
        click_label(&mut harness, "Показывать сетку");
        close_menu(&mut harness);
        assert_eq!(dots(&harness), 0);
        open_display_menu(&mut harness, "Сетка");
        click_label(&mut harness, "Показывать сетку");
        close_menu(&mut harness);
        assert!(dots(&harness) > 0);
        click_label(&mut harness, "Город");
        click_label(&mut harness, "Москва");
        assert!(dots(&harness) > 0);
        open_display_menu(&mut harness, "Сетка");
        assert_eq!(
            harness
                .get_by_role_and_label(egui::accesskit::Role::Slider, "Базовый шаг")
                .accesskit_node()
                .numeric_value(),
            Some(base_step)
        );
        assert_eq!(
            harness
                .get_by_role_and_label(egui::accesskit::Role::Slider, "Интервал на экране")
                .accesskit_node()
                .numeric_value(),
            Some(spacing)
        );
        click_label(&mut harness, "Линии сетки");
        close_menu(&mut harness);
        assert_eq!(dots(&harness), 0);
        click_label(&mut harness, "Режимы привязки");
        assert_eq!(
            harness
                .get_by_label("К базовой сетке")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        click_label(&mut harness, "К адаптивной сетке");
        assert_eq!(
            harness
                .get_by_label("К базовой сетке")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::False)
        );
        assert_eq!(
            harness
                .get_by_label("К адаптивной сетке")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
    }
}

#[test]
fn frame_time_graph_stays_in_the_bottom_right_of_the_map() {
    for width in [360.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 844.0));
        harness.run_steps(5);
        let map = harness.get_by_label("Схема метро Санкт-Петербурга").rect();
        let shapes = &harness.output().shapes;
        let label = shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text.ends_with(" ms") => Some(text),
                _ => None,
            })
            .expect("Frame time must be displayed");
        assert_eq!(label.fallback_color, egui::Color32::from_gray(140));
        assert!(map.contains_rect(label.visual_bounding_rect()));
        assert!((label.visual_bounding_rect().right() - (map.right() - 8.0)).abs() < 2.0);
        let mut segments = shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::LineSegment { points, stroke }
                    if stroke.width == 1.0 && stroke.color == egui::Color32::from_gray(140) =>
                {
                    Some(points)
                }
                _ => None,
            })
            .peekable();
        assert!(segments.peek().is_some());
        for points in segments {
            for point in points {
                assert!(map.contains(*point));
                assert!(point.x > map.right() - 196.0);
                assert!(point.y > map.bottom() - 80.0);
            }
        }
    }
}

#[test]
fn crossing_gaps_toggle_rendering_and_remain_enabled_after_city_and_routing_changes() {
    for width in [360.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 844.0));
        harness.run();
        let edge_fragments = |harness: &Harness<'_, Demo>| {
            harness.output().shapes.iter().filter(|shape| {
                matches!(&shape.shape, egui::Shape::LineSegment { stroke, .. } if stroke.width == 4.0)
            }).count()
        };
        open_display_menu(&mut harness, "Пересечения");
        assert_eq!(
            harness.get_by_label("Разрыв").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        close_menu(&mut harness);
        let interrupted = edge_fragments(&harness);
        open_display_menu(&mut harness, "Пересечения");
        harness.get_by_label("Без оформления").scroll_to_me();
        harness.run();
        click_label(&mut harness, "Без оформления");
        assert!(harness.get_by_label("По ID").accesskit_node().is_disabled());
        close_menu(&mut harness);
        let continuous = edge_fragments(&harness);
        assert!(interrupted > continuous);
        open_display_menu(&mut harness, "Пересечения");
        click_label(&mut harness, "Разрыв");
        close_menu(&mut harness);
        assert_eq!(edge_fragments(&harness), interrupted);
        open_display_menu(&mut harness, "Линии и подписи");
        click_label(&mut harness, "Кривые");
        open_display_menu(&mut harness, "Пересечения");
        assert!(
            !harness
                .get_by_label("Разрыв")
                .accesskit_node()
                .is_disabled()
        );
        click_label(&mut harness, "Дуга-мостик");
        close_menu(&mut harness);
        assert!(harness.output().shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Path(path) if path.points.len() == 25)
        }));
        open_display_menu(&mut harness, "Пересечения");
        click_label(&mut harness, "Разрыв");
        open_display_menu(&mut harness, "Линии и подписи");
        click_label(&mut harness, "Прямые");
        close_menu(&mut harness);
        assert_eq!(edge_fragments(&harness), interrupted);
        click_label(&mut harness, "Город");
        click_label(&mut harness, "Москва");
        open_display_menu(&mut harness, "Пересечения");
        assert_eq!(
            harness.get_by_label("Разрыв").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        harness.get_by_label("Без оформления").scroll_to_me();
        harness.run();
        click_label(&mut harness, "Без оформления");
        assert_eq!(
            harness
                .get_by_label("Без оформления")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
    }
}

#[test]
fn edge_routing_selector_updates_rendering_and_persists_when_switching_city() {
    for width in [360.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 844.0));
        harness.run();
        let has_curves = |harness: &Harness<'_, Demo>| {
            harness
                .output()
                .shapes
                .iter()
                .any(|shape| matches!(shape.shape, egui::Shape::QuadraticBezier(_)))
        };
        assert!(!has_curves(&harness));
        open_display_menu(&mut harness, "Линии и подписи");
        for label in [
            "Кривые",
            "Углы 90°",
            "Углы 45°",
            "Углы 15°",
            "Прямые",
            "Кривые",
        ] {
            let rect = harness.get_by_label(label).rect();
            assert!(rect.left() >= 0.0 && rect.right() <= width);
            click_label(&mut harness, label);
            assert_eq!(
                harness.get_by_label(label).accesskit_node().toggled(),
                Some(egui::accesskit::Toggled::True)
            );
            assert_eq!(has_curves(&harness), label == "Кривые");
        }
        close_menu(&mut harness);
        click_label(&mut harness, "Город");
        click_label(&mut harness, "Москва");
        open_display_menu(&mut harness, "Линии и подписи");
        assert_eq!(
            harness.get_by_label("Кривые").accesskit_node().toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        assert!(has_curves(&harness));
    }
}

#[test]
fn crossing_settings_switch_to_bridges_and_keep_mode_after_city_change() {
    for width in [360.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 844.0));
        harness.run();
        open_display_menu(&mut harness, "Пересечения");
        click_label(&mut harness, "Дуга-мостик");
        assert!(
            harness
                .query_by_role_and_label(egui::accesskit::Role::Slider, "Высота дуги")
                .is_some()
        );
        let height_slider =
            harness.get_by_role_and_label(egui::accesskit::Role::Slider, "Высота дуги");
        assert!(height_slider.rect().left() >= 0.0 && height_slider.rect().right() <= width);
        let height = height_slider.accesskit_node().numeric_value().unwrap();
        height_slider.focus();
        harness.run();
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        assert!(
            harness
                .get_by_role_and_label(egui::accesskit::Role::Slider, "Высота дуги")
                .accesskit_node()
                .numeric_value()
                .unwrap()
                > height
        );
        harness.get_by_label("Справа по ходу ребра").scroll_to_me();
        harness.run();
        click_label(&mut harness, "Справа по ходу ребра");
        assert_eq!(
            harness
                .get_by_label("Справа по ходу ребра")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
        harness
            .get_by_label("Размер по толщине линий")
            .scroll_to_me();
        harness.run();
        click_label(&mut harness, "Размер по толщине линий");
        assert!(
            harness
                .query_by_role_and_label(egui::accesskit::Role::Slider, "Отступ от линий")
                .is_some()
        );
        harness.key_press(egui::Key::Escape);
        harness.run();
        let arc_count = |harness: &Harness<'_, Demo>| {
            harness.output().shapes.iter().filter(|shape| {
            matches!(&shape.shape, egui::Shape::Path(path) if path.points.len() == 25)
        }).count()
        };
        let bridges = arc_count(&harness);
        assert!(bridges > 0);
        open_display_menu(&mut harness, "Пересечения");
        harness.get_by_label("Без оформления").scroll_to_me();
        harness.run();
        click_label(&mut harness, "Без оформления");
        close_menu(&mut harness);
        assert_eq!(arc_count(&harness), 0);
        open_display_menu(&mut harness, "Пересечения");
        click_label(&mut harness, "Дуга-мостик");
        close_menu(&mut harness);
        assert_eq!(arc_count(&harness), bridges);
        click_label(&mut harness, "Город");
        click_label(&mut harness, "Москва");
        open_display_menu(&mut harness, "Пересечения");
        assert_eq!(
            harness
                .get_by_label("Дуга-мостик")
                .accesskit_node()
                .toggled(),
            Some(egui::accesskit::Toggled::True)
        );
    }
}

#[test]
fn selected_edge_crossing_override_can_be_enabled_and_reset() {
    let mut harness = desktop();
    harness.run();
    let viewport = harness.get_by_label("Схема метро Санкт-Петербурга").rect();
    let nodes: Vec<_> = harness
        .get_all_by_role(egui::accesskit::Role::Button)
        .map(|node| node.rect().expand(8.0))
        .collect();
    let point = harness
        .output()
        .shapes
        .iter()
        .filter_map(|shape| {
            let egui::Shape::LineSegment { points, stroke } = &shape.shape else {
                return None;
            };
            let middle = points[0].lerp(points[1], 0.5);
            (stroke.width == 4.0
                && viewport.contains(middle)
                && !nodes.iter().any(|rect| rect.contains(middle)))
            .then_some((points[0].distance(points[1]), middle))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap()
        .1;
    harness.hover_at(point);
    for pressed in [true, false] {
        harness.event(pointer_button(
            point,
            egui::PointerButton::Primary,
            pressed,
            egui::Modifiers::NONE,
        ));
    }
    harness.run();
    open_display_menu(&mut harness, "Пересечения");
    harness.get_by_label("Выбранное ребро").scroll_to_me();
    harness.run();
    click_label(&mut harness, "Выбранное ребро");
    harness
        .get_by_label("Особое оформление ребра")
        .scroll_to_me();
    harness.run();
    click_label(&mut harness, "Особое оформление ребра");
    assert_eq!(
        harness
            .get_by_label("Особое оформление ребра")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::True)
    );
    harness
        .get_by_label("Сбросить настройки ребра")
        .scroll_to_me();
    harness.run();
    click_label(&mut harness, "Сбросить настройки ребра");
    harness
        .get_by_label("Особое оформление ребра")
        .scroll_to_me();
    harness.run();
    assert_eq!(
        harness
            .get_by_label("Особое оформление ребра")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );
}

#[test]
fn label_visibility_and_zoom_threshold_update_the_map_and_survive_city_changes() {
    let mut harness = demo_harness(Vec2::new(390.0, 844.0));
    harness.run();
    let has_station_labels = |harness: &Harness<'_, Demo>| {
        harness.output().shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == "Беговая")
        })
    };
    assert!(!has_station_labels(&harness));
    open_display_menu(&mut harness, "Линии и подписи");
    let slider =
        harness.get_by_role_and_label(egui::accesskit::Role::Slider, "Показывать от масштаба");
    let point = slider.rect().left_center();
    harness.hover_at(point);
    for pressed in [true, false] {
        harness.event(pointer_button(
            point,
            egui::PointerButton::Primary,
            pressed,
            egui::Modifiers::NONE,
        ));
    }
    harness.run();
    assert_eq!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Показывать от масштаба")
            .accesskit_node()
            .numeric_value(),
        Some(0.0)
    );
    close_menu(&mut harness);
    assert!(has_station_labels(&harness));
    assert!(
        harness
            .query_by_label_contains("Приблизьте схему")
            .is_none()
    );
    open_display_menu(&mut harness, "Линии и подписи");
    click_label(&mut harness, "Подписи станций");
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Показывать от масштаба")
            .accesskit_node()
            .is_disabled()
    );
    close_menu(&mut harness);
    assert!(!has_station_labels(&harness));
    click_label(&mut harness, "Город");
    click_label(&mut harness, "Москва");
    open_display_menu(&mut harness, "Линии и подписи");
    assert_eq!(
        harness
            .get_by_label("Подписи станций")
            .accesskit_node()
            .toggled(),
        Some(egui::accesskit::Toggled::False)
    );
    assert_eq!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Показывать от масштаба")
            .accesskit_node()
            .numeric_value(),
        Some(0.0)
    );
}

#[test]
fn corner_radius_control_updates_geometry_and_persists_across_city_changes() {
    for width in [360.0, 1440.0] {
        let mut harness = demo_harness(Vec2::new(width, 844.0));
        harness.run();
        open_display_menu(&mut harness, "Линии и подписи");
        for label in ["Прямые", "Кривые"] {
            click_label(&mut harness, label);
            assert!(
                harness
                    .get_by_role_and_label(egui::accesskit::Role::Slider, "Радиус скругления")
                    .accesskit_node()
                    .is_disabled()
            );
        }
        click_label(&mut harness, "Углы 45°");
        let slider =
            harness.get_by_role_and_label(egui::accesskit::Role::Slider, "Радиус скругления");
        assert!(!slider.accesskit_node().is_disabled());
        slider.scroll_to_me();
        harness.run();
        harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Радиус скругления")
            .focus();
        harness.run();
        harness.key_press(egui::Key::ArrowRight);
        harness.run();
        let radius = harness
            .get_by_role_and_label(egui::accesskit::Role::Slider, "Радиус скругления")
            .accesskit_node()
            .numeric_value()
            .unwrap();
        assert!(radius > 0.0);
        close_menu(&mut harness);
        click_label(&mut harness, "Город");
        click_label(&mut harness, "Москва");
        open_display_menu(&mut harness, "Линии и подписи");
        assert_eq!(
            harness
                .get_by_role_and_label(egui::accesskit::Role::Slider, "Радиус скругления")
                .accesskit_node()
                .numeric_value(),
            Some(radius)
        );
        click_label(&mut harness, "Углы 15°");
        assert_eq!(
            harness
                .get_by_role_and_label(egui::accesskit::Role::Slider, "Радиус скругления")
                .accesskit_node()
                .numeric_value(),
            Some(radius)
        );
    }
}
