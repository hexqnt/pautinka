use egui::Ui;
use pautinka::egui::{
    BridgeSide, CrossingAngle, CrossingMode, CrossingOptions, CrossingOrder, CrossingPoints,
    CrossingSize, ElementId,
};

use crate::i18n::{Language, Text};

use super::{Demo, labeled_slider};

impl Demo {
    pub(super) fn crossing_controls(&mut self, ui: &mut Ui) {
        let language = self.language();
        ui.scope(|ui| {
            let options = &mut self.canvas.style.crossings;
            mode_controls(ui, &mut options.mode, language);
            ui.add_enabled_ui(options.mode != CrossingMode::None, |ui| {
                order_controls(ui, &mut options.order, language);
            });
            appearance_controls(ui, options, language);
            ui.separator();
            egui::CollapsingHeader::new(language.text(Text::SelectedEdge))
                .id_salt("selected-edge")
                .show(ui, |ui| {
                    self.selected_edge_crossing_controls(ui);
                });
        });
    }

    fn selected_edge_crossing_controls(&mut self, ui: &mut Ui) {
        let language = self.language();
        let Some(ElementId::Edge(id)) = self.canvas.selected_id() else {
            ui.weak(language.text(Text::SelectEdgeHint));
            return;
        };
        let previous = self.canvas.edge_crossings(id);
        let mut settings = previous.unwrap_or_default();
        ui.horizontal(|ui| {
            let label = ui.label(language.text(Text::EdgePriority));
            ui.add(egui::DragValue::new(&mut settings.priority).range(-100..=100))
                .labelled_by(label.id);
        })
        .response
        .on_hover_text(language.text(Text::EdgePriorityHint));
        let mut custom = settings.options.is_some();
        if ui
            .checkbox(&mut custom, language.text(Text::CustomEdgeStyle))
            .changed()
        {
            settings.options = custom.then_some(self.canvas.style.crossings);
        }
        if let Some(options) = &mut settings.options {
            ui.push_id("edge_crossings", |ui| {
                mode_controls(ui, &mut options.mode, language);
                appearance_controls(ui, options, language);
            });
        }
        if ui.button(language.text(Text::ResetEdge)).clicked() {
            self.canvas.set_edge_crossings(id, None);
        } else if settings != previous.unwrap_or_default() {
            self.canvas.set_edge_crossings(id, Some(settings));
        }
    }
}

fn mode_controls(ui: &mut Ui, mode: &mut CrossingMode, language: Language) {
    ui.label(language.text(Text::CrossingStyle));
    for (value, label) in [
        (CrossingMode::None, language.text(Text::NoCrossingStyle)),
        (CrossingMode::Gap, language.text(Text::Gap)),
        (CrossingMode::Bridge, language.text(Text::Bridge)),
    ] {
        ui.radio_value(mode, value, label);
    }
}

fn order_controls(ui: &mut Ui, order: &mut CrossingOrder, language: Language) {
    ui.separator();
    ui.label(language.text(Text::TopEdge));
    for (value, label) in [
        (CrossingOrder::StableId, language.text(Text::ById)),
        (
            CrossingOrder::HorizontalFirst,
            language.text(Text::HorizontalFirst),
        ),
        (
            CrossingOrder::VerticalFirst,
            language.text(Text::VerticalFirst),
        ),
    ] {
        ui.radio_value(order, value, label);
    }
}

fn appearance_controls(ui: &mut Ui, options: &mut CrossingOptions, language: Language) {
    ui.add_enabled_ui(options.mode != CrossingMode::None, |ui| {
        let mut auto = matches!(options.size, CrossingSize::Auto { .. });
        if ui
            .checkbox(&mut auto, language.text(Text::AutoSize))
            .changed()
        {
            options.size = if auto {
                CrossingSize::Auto {
                    padding: CrossingPoints::new(3.0).unwrap(),
                }
            } else {
                CrossingOptions::default().size
            };
        }
        match &mut options.size {
            CrossingSize::Fixed(size) => {
                points_slider(ui, size, language.text(Text::Width), 1.0..=64.0);
            }
            CrossingSize::Auto { padding } => {
                points_slider(ui, padding, language.text(Text::LinePadding), 0.0..=32.0);
            }
        }
        if options.mode == CrossingMode::Bridge {
            points_slider(
                ui,
                &mut options.bridge_height,
                language.text(Text::BridgeHeight),
                1.0..=64.0,
            );
            ui.label(language.text(Text::BridgeSide));
            for (side, label) in [
                (BridgeSide::Auto, language.text(Text::Automatic)),
                (BridgeSide::Left, language.text(Text::LeftOfEdge)),
                (BridgeSide::Right, language.text(Text::RightOfEdge)),
            ] {
                ui.radio_value(&mut options.bridge_side, side, label);
            }
        }
        egui::CollapsingHeader::new(language.text(Text::Advanced))
            .id_salt("advanced-crossings")
            .show(ui, |ui| {
                let mut angle = options.min_angle.degrees();
                if labeled_slider(
                    ui,
                    language.text(Text::MinAngle),
                    egui::Slider::new(&mut angle, 1.0..=90.0).suffix("°"),
                )
                .changed()
                {
                    options.min_angle = CrossingAngle::new(angle).unwrap();
                }
                points_slider(
                    ui,
                    &mut options.clearance,
                    language.text(Text::BendClearance),
                    0.0..=32.0,
                );
            });
    });
}

fn points_slider(
    ui: &mut Ui,
    points: &mut CrossingPoints,
    label: &str,
    range: std::ops::RangeInclusive<f32>,
) {
    let mut value = points.get();
    if labeled_slider(
        ui,
        label,
        egui::Slider::new(&mut value, range).suffix(" pt"),
    )
    .changed()
    {
        *points = CrossingPoints::new(value).unwrap();
    }
}
