mod crossings;
mod grid;

use egui::{Color32, Ui};
use pautinka::egui::{CornerRadius, GridSnapMode, NavigatorCorner, RoutingMode, SnapKinds};

use crate::i18n::{Language, Text};

use super::Demo;

impl Demo {
    pub(super) fn rendering_controls(&mut self, ui: &mut Ui) {
        let language = self.language();
        ui.horizontal_wrapped(|ui| {
            settings_menu(
                ui,
                language.text(Text::LinesLabels),
                language.text(Text::LinesDescription),
                |ui| {
                    self.routing_selector(ui);
                    ui.separator();
                    let mut labels = self.canvas.labels();
                    if ui
                        .checkbox(&mut labels, language.text(Text::StationLabels))
                        .changed()
                    {
                        self.canvas.set_labels(labels);
                    }
                    ui.add_enabled_ui(labels, |ui| {
                        labeled_slider(
                            ui,
                            language.text(Text::LabelZoom),
                            egui::Slider::new(&mut self.canvas.style.label_min_zoom, 0.0..=1.0)
                                .suffix("×"),
                        );
                        ui.weak(language.text(Text::SelectedLabelHint));
                    });
                },
            );
            settings_menu(
                ui,
                language.text(Text::Crossings),
                language.text(Text::CrossingsDescription),
                |ui| {
                    self.crossing_controls(ui);
                },
            );
            settings_menu(
                ui,
                language.text(Text::Navigation),
                language.text(Text::NavigationDescription),
                |ui| {
                    self.navigator_controls(ui);
                },
            );
            settings_menu(
                ui,
                language.text(Text::Grid),
                language.text(Text::GridDescription),
                |ui| {
                    self.grid_controls(ui);
                },
            );
        });
    }

    fn navigator_controls(&mut self, ui: &mut Ui) {
        let language = self.language();
        let options = &mut self.canvas.options.navigator;
        ui.checkbox(&mut options.enabled, language.text(Text::Minimap));
        ui.add_enabled_ui(options.enabled, |ui| {
            ui.label(language.text(Text::MinimapPosition));
            for corner in [
                NavigatorCorner::TopLeft,
                NavigatorCorner::TopRight,
                NavigatorCorner::BottomLeft,
                NavigatorCorner::BottomRight,
            ] {
                ui.radio_value(
                    &mut options.corner,
                    corner,
                    navigator_corner_label(corner, language),
                );
            }
            ui.weak(language.text(Text::MinimapHint));
        });
        ui.separator();
        if ui.button(language.text(Text::FitMap)).clicked() {
            self.canvas.camera.fit = true;
        }
        ui.weak(language.text(Text::NavigationHint));
    }

    fn routing_selector(&mut self, ui: &mut Ui) {
        let language = self.language();
        ui.strong(language.text(Text::Geometry));
        let mut mode = self.metro.routing();
        for choice in [
            RoutingMode::Straight,
            RoutingMode::Curved,
            RoutingMode::Orthogonal,
            RoutingMode::Octilinear,
            RoutingMode::Angular15,
        ] {
            ui.radio_value(&mut mode, choice, routing_label(choice, language));
        }
        if let Err(error) = self.metro.set_routing(mode) {
            ui.colored_label(Color32::RED, error.to_string());
        }
        self.corner_radius_control(ui);
    }

    fn corner_radius_control(&mut self, ui: &mut Ui) {
        let language = self.language();
        let mut radius = self.metro.corner_radius().get();
        ui.add_enabled_ui(
            !matches!(
                self.metro.routing(),
                RoutingMode::Straight | RoutingMode::Curved
            ),
            |ui| {
                if labeled_slider(
                    ui,
                    language.text(Text::CornerRadius),
                    egui::Slider::new(&mut radius, 0.0..=100.0),
                )
                .changed()
                    && let Err(error) = CornerRadius::try_from(radius)
                        .and_then(|radius| self.metro.set_corner_radius(radius))
                {
                    ui.colored_label(Color32::RED, error.to_string());
                }
            },
        );
    }

    pub(super) fn footer(&mut self, ui: &mut Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let mut language = self.language();
            let response = egui::ComboBox::from_id_salt("language")
                .width(48.0)
                .selected_text(match language {
                    Language::English => "EN",
                    Language::Russian => "RU",
                })
                .show_ui(ui, |ui| {
                    for choice in Language::ALL {
                        ui.selectable_value(&mut language, choice, choice.name());
                    }
                })
                .response;
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_label("Language / Язык"));
            if language != self.language() {
                self.set_language(language);
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Title(language.app_title().into()));
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                self.editing_controls(ui);
            });
        });
    }

    fn editing_controls(&mut self, ui: &mut Ui) {
        let language = self.language();
        ui.spacing_mut().interact_size.y =
            ui.text_style_height(&egui::TextStyle::Button) + 2.0 * ui.spacing().button_padding.y;
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(
                &mut self.canvas.options.interaction.snapping,
                language.text(Text::Snapping),
            );
            ui.add_enabled_ui(self.canvas.options.interaction.snapping, |ui| {
                egui::containers::menu::MenuButton::from_button(
                    egui::Button::new(language.text(Text::SnapModes))
                        .wrap_mode(egui::TextWrapMode::Extend),
                )
                .config(
                    egui::containers::menu::MenuConfig::new()
                        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside),
                )
                .ui(ui, |ui| {
                    for (kind, label) in [
                        (SnapKinds::HORIZONTAL, language.text(Text::Horizontal)),
                        (SnapKinds::VERTICAL, language.text(Text::Vertical)),
                        (SnapKinds::DIAGONAL_UP, language.text(Text::DiagonalUp)),
                        (SnapKinds::DIAGONAL_DOWN, language.text(Text::DiagonalDown)),
                    ] {
                        let mut enabled = self.canvas.options.snapping.kinds.contains(kind);
                        if ui.checkbox(&mut enabled, label).changed() {
                            self.canvas.options.snapping.kinds.set(kind, enabled);
                        }
                    }
                    ui.separator();
                    for (mode, label) in [
                        (GridSnapMode::Disabled, language.text(Text::NoGridSnap)),
                        (
                            GridSnapMode::Adaptive,
                            language.text(Text::AdaptiveGridSnap),
                        ),
                        (GridSnapMode::Fixed, language.text(Text::FixedGridSnap)),
                    ] {
                        ui.radio_value(&mut self.canvas.options.grid_snap, mode, label);
                    }
                });
            });
        });
    }
}

fn settings_menu(ui: &mut Ui, title: &str, description: &str, contents: impl FnOnce(&mut Ui)) {
    egui::containers::menu::MenuButton::new(title)
        .config(
            egui::containers::menu::MenuConfig::new()
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .style(|style: &mut egui::Style| {
                    style.visuals = egui::Visuals::light();
                    style.visuals.override_text_color = Some(crate::render::INK);
                }),
        )
        .ui(ui, |ui| {
            let screen = ui.ctx().content_rect();
            ui.set_width((screen.width() - 32.0).min(300.0));
            ui.spacing_mut().slider_width = 210.0;
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .max_height((screen.height() - 100.0).min(520.0))
                .show(ui, |ui| {
                    ui.strong(title);
                    ui.label(description);
                    ui.separator();
                    contents(ui);
                });
        });
}

fn labeled_slider(ui: &mut Ui, text: &str, slider: egui::Slider<'_>) -> egui::Response {
    let label = ui.label(text);
    ui.add(slider).labelled_by(label.id)
}

pub(super) fn combo_box<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    label: &str,
    value: &mut T,
    choices: impl IntoIterator<Item = T>,
    text: impl Fn(T) -> &'static str,
) {
    let response = egui::ComboBox::from_id_salt(id)
        .selected_text(text(*value))
        .show_ui(ui, |ui| {
            for choice in choices {
                ui.selectable_value(value, choice, text(choice));
            }
        })
        .response;
    ui.ctx()
        .accesskit_node_builder(response.id, |node| node.set_label(label));
}

const fn routing_label(mode: RoutingMode, language: Language) -> &'static str {
    match mode {
        RoutingMode::Straight => language.text(Text::Straight),
        RoutingMode::Curved => language.text(Text::Curved),
        RoutingMode::Orthogonal => language.text(Text::Orthogonal),
        RoutingMode::Octilinear => language.text(Text::Octilinear),
        RoutingMode::Angular15 => language.text(Text::Angular15),
    }
}

const fn navigator_corner_label(corner: NavigatorCorner, language: Language) -> &'static str {
    match corner {
        NavigatorCorner::TopLeft => language.text(Text::TopLeft),
        NavigatorCorner::TopRight => language.text(Text::TopRight),
        NavigatorCorner::BottomLeft => language.text(Text::BottomLeft),
        NavigatorCorner::BottomRight => language.text(Text::BottomRight),
    }
}
