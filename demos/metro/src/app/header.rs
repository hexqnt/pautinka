use egui::{Color32, RichText, Vec2};

use crate::i18n::Text;
use crate::metro::City;

use super::{Demo, controls::combo_box};

impl Demo {
    pub(super) fn header(&mut self, ui: &mut egui::Ui, compact: bool) {
        let language = self.language();
        egui::Panel::top("masthead")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(17, 29, 40))
                    .inner_margin(16.0),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if !compact {
                        self.explorer_toggle(ui);
                        self.city_selector(ui);
                        let search_width = (ui.available_width() - 100.0).clamp(120.0, 480.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(search_width, ui.spacing().interact_size.y),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| self.station_search(ui),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        Self::github_link(ui);
                        if compact && ui.button(language.text(Text::Stations)).clicked() {
                            self.show_explorer = !self.show_explorer;
                        }
                    });
                });
                if compact {
                    ui.add_space(8.0);
                    self.city_selector(ui);
                    ui.horizontal(|ui| self.station_search(ui));
                }
            });
    }

    fn explorer_toggle(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        let (icon, label) = if self.explorer_collapsed {
            ("▶", language.text(Text::ShowSidebar))
        } else {
            ("◀", language.text(Text::HideSidebar))
        };
        let response = ui.button(icon).on_hover_text(label);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), label));
        if response.clicked() {
            self.explorer_collapsed = !self.explorer_collapsed;
        }
    }

    fn github_link(ui: &mut egui::Ui) {
        const URL: &str = "https://github.com/hexqnt/pautinka";
        let image = egui::Image::new(egui::include_image!("../../assets/brand-github.svg"))
            .fit_to_exact_size(Vec2::splat(20.0));
        let response = ui
            .add(
                egui::Button::image_and_text(
                    image,
                    RichText::new("pautinka").color(ui.visuals().hyperlink_color),
                )
                .image_tint_follows_text_color(true)
                .frame(false),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(URL);
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Link, ui.is_enabled(), "pautinka")
        });
        if response.clicked() || response.clicked_with_open_in_background() {
            ui.open_url(egui::OpenUrl {
                url: URL.into(),
                new_tab: response.clicked_with_open_in_background(),
            });
        }
    }

    fn city_selector(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        let mut city = self.city;
        combo_box(
            ui,
            "city",
            language.text(Text::City),
            &mut city,
            City::ALL,
            |city| language.city_name(city),
        );
        if city != self.city {
            self.switch_city(city);
        }
    }

    fn station_search(&mut self, ui: &mut egui::Ui) {
        if let Some(station) = self.search.show(ui, &self.metro) {
            self.select(station);
        }
    }
}
