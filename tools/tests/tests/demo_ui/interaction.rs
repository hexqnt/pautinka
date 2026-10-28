use super::*;

fn begin_drag(harness: &mut Harness<'_, Demo>, from: egui::Pos2, to: egui::Pos2) {
    harness.hover_at(from);
    harness.event(pointer_button(
        from,
        egui::PointerButton::Primary,
        true,
        egui::Modifiers::NONE,
    ));
    harness.step();
    harness.event(egui::Event::PointerMoved(to));
    harness.run();
}

fn release_drag(harness: &mut Harness<'_, Demo>, at: egui::Pos2) {
    harness.event(pointer_button(
        at,
        egui::PointerButton::Primary,
        false,
        egui::Modifiers::NONE,
    ));
    harness.run();
}

#[test]
fn station_drag_is_enabled_by_default_and_can_be_cancelled() {
    let mut harness = desktop();
    harness.run();
    click_label(&mut harness, "Привязка");
    let from = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
        .rect()
        .center();
    let to = from + Vec2::new(72.0, 40.0);
    begin_drag(&mut harness, from, to);
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center()
            .distance(to)
            < 0.1
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    release_drag(&mut harness, to);
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center()
            .distance(from)
            < 0.1
    );
    begin_drag(&mut harness, from, to);
    release_drag(&mut harness, to);
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center()
            .distance(to)
            < 0.1
    );
}

#[test]
fn station_drag_uses_selected_vertical_snapping() {
    let mut harness = desktop();
    harness.run();
    click_label(&mut harness, "Режимы привязки");
    click_label(&mut harness, "Горизонталь");
    harness.key_press(egui::Key::Escape);
    harness.run();
    let from = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
        .rect()
        .center();
    let to = from + Vec2::new(3.0, 70.0);
    begin_drag(&mut harness, from, to);
    let moved = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
        .rect()
        .center();
    assert!((moved.x - from.x).abs() < 0.1);
    assert!((moved.y - to.y).abs() < 0.1);
    release_drag(&mut harness, to);
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center()
            .distance(moved)
            < 0.1
    );
}

#[test]
fn diagonal_snapping_draws_guides_for_the_selected_direction() {
    for (label, sign) in [("Диагональ / (45°)", -1.0), ("Диагональ \\ (45°)", 1.0)]
    {
        let mut harness = desktop();
        harness.run();
        click_label(&mut harness, "Режимы привязки");
        for mode in ["Горизонталь", "Вертикаль", label] {
            click_label(&mut harness, mode);
        }
        harness.key_press(egui::Key::Escape);
        harness.run();
        let from = harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center();
        let source = harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Зенит")
            .rect()
            .center();
        let to = source + Vec2::new(40.0, 38.0 * sign);
        begin_drag(&mut harness, from, to);
        let guides: Vec<_> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::LineSegment { points, stroke }
                    if stroke.color == egui::Color32::from_rgb(186, 112, 34) =>
                {
                    Some(points)
                }
                _ => None,
            })
            .collect();
        assert_ne!(guides, [] as [&[egui::Pos2; 2]; 0]);
        for [start, end] in guides {
            let delta = *end - *start;
            assert!((delta.y - sign * delta.x).abs() < 0.01);
        }
        release_drag(&mut harness, to);
    }
}

#[test]
fn selected_stations_move_as_a_group_and_escape_restores_both() {
    let mut harness = desktop();
    harness.run();
    click_label(&mut harness, "Привязка");
    click_role_label(&mut harness, egui::accesskit::Role::Button, "Беговая");
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Зенит")
        .click_modifiers(egui::Modifiers::COMMAND);
    harness.run();
    let first = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
        .rect()
        .center();
    let second = harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Зенит")
        .rect()
        .center();
    let delta = Vec2::new(80.0, 40.0);
    begin_drag(&mut harness, first, first + delta);
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center()
            .distance(first + delta)
            < 0.1
    );
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Зенит")
            .rect()
            .center()
            .distance(second + delta)
            < 0.1
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    release_drag(&mut harness, first + delta);
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Беговая")
            .rect()
            .center()
            .distance(first)
            < 0.1
    );
    assert!(
        harness
            .get_by_role_and_label(egui::accesskit::Role::Button, "Зенит")
            .rect()
            .center()
            .distance(second)
            < 0.1
    );
}

#[test]
fn navigator_can_be_enabled_repositioned_and_disabled() {
    let mut harness = desktop();
    harness.run();
    let panel = harness
        .get_by_role_and_label(egui::accesskit::Role::Unknown, "Navigator")
        .rect();
    let viewport = harness.get_by_label("Схема метро Санкт-Петербурга").rect();
    assert_eq!(panel.right(), viewport.right() - 12.0);
    assert_eq!(panel.top(), viewport.top() + 12.0);
    open_display_menu(&mut harness, "Навигация");
    click_label(&mut harness, "Мини-карта");
    assert!(
        harness
            .query_by_role_and_label(egui::accesskit::Role::Unknown, "Navigator")
            .is_none()
    );
    click_label(&mut harness, "Мини-карта");
    click_label(&mut harness, "Слева снизу");
    let panel = harness
        .get_by_role_and_label(egui::accesskit::Role::Unknown, "Navigator")
        .rect();
    let viewport = harness.get_by_label("Схема метро Санкт-Петербурга").rect();
    assert!(panel.center().x < viewport.center().x);
    assert!(panel.center().y > viewport.center().y);
    click_label(&mut harness, "Мини-карта");
    assert!(
        harness
            .query_by_role_and_label(egui::accesskit::Role::Unknown, "Navigator")
            .is_none()
    );
}
