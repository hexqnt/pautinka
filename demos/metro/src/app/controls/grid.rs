use egui::Ui;
use pautinka::egui::{GridKind, GridOptions};

use crate::i18n::Text;

use super::{Demo, labeled_slider};

impl Demo {
    pub(super) fn grid_controls(&mut self, ui: &mut Ui) {
        let language = self.language();
        let options = self.canvas.options.grid.get_or_insert_default();
        ui.checkbox(&mut options.visible, language.text(Text::ShowGrid));
        ui.weak(language.text(Text::GridSnappingHint));
        ui.add_enabled_ui(options.visible, |ui| {
            ui.strong(language.text(Text::Pattern));
            ui.radio_value(
                &mut options.kind,
                GridKind::Lines,
                language.text(Text::GridLines),
            );
            ui.radio_value(
                &mut options.kind,
                GridKind::Dots,
                language.text(Text::GridDots),
            );
            ui.separator();
            ui.strong(language.text(Text::GridSpacing));
            let mut base_step = options.base_step();
            let mut spacing = options.min_spacing();
            labeled_slider(
                ui,
                language.text(Text::BaseStep),
                egui::Slider::new(&mut base_step, 0.1..=1000.0).logarithmic(true),
            );
            labeled_slider(
                ui,
                language.text(Text::ScreenSpacing),
                egui::Slider::new(&mut spacing, GridOptions::MIN_SPACING..=64.0).suffix(" pt"),
            );
            ui.separator();
            ui.strong(language.text(Text::Appearance));
            let mut minor = options.minor_width();
            let mut major = options.major_width();
            labeled_slider(
                ui,
                language.text(Text::MinorSize),
                egui::Slider::new(&mut minor, 0.5..=3.0).suffix(" pt"),
            );
            labeled_slider(
                ui,
                language.text(Text::MajorSize),
                egui::Slider::new(&mut major, 0.5..=4.0).suffix(" pt"),
            );
            ui.horizontal(|ui| {
                ui.label(language.text(Text::MinorMarks));
                ui.color_edit_button_srgba(&mut options.minor_color);
            });
            ui.horizontal(|ui| {
                ui.label(language.text(Text::MajorMarks));
                ui.color_edit_button_srgba(&mut options.major_color);
            });
            if let Some(updated) = options
                .with_spacing(base_step, spacing)
                .and_then(|grid| grid.with_widths(minor, major))
            {
                *options = updated;
            }
        });
    }
}
