use egui::RichText;

use crate::render::ACCENT;

use crate::i18n::Text;

use super::Demo;

impl Demo {
    fn lines(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        ui.label(
            RichText::new(language.text(Text::LinesHeading))
                .size(11.0)
                .color(ACCENT)
                .strong(),
        );
        if ui
            .selectable_label(self.line.is_none(), language.text(Text::AllLines))
            .clicked()
        {
            self.line = None;
        }
        for &line in self.city.lines() {
            let label = format!("{}   {}", line.number(), language.line_name(line));
            if ui
                .selectable_label(
                    self.line == Some(line),
                    RichText::new(label).color(line.color()).size(12.0),
                )
                .clicked()
            {
                self.line = Some(line);
                self.canvas.camera.fit = true;
                self.clear_route();
            }
        }
    }

    pub(super) fn explorer(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        egui::ScrollArea::vertical()
            .id_salt("explorer-scroll")
            .show(ui, |ui| {
                ui.label(
                    RichText::new(language.text(Text::MetroHeading))
                        .color(ACCENT)
                        .size(11.0)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(language.city_title(self.city))
                        .size(34.0)
                        .strong(),
                );
                ui.label(language.text(Text::ExploreHint));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}", self.metro.stations.len()))
                            .size(26.0)
                            .color(ACCENT),
                    );
                    ui.weak(language.text(Text::StationsUnit));
                    ui.separator();
                    ui.label(
                        RichText::new(self.city.lines().len().to_string())
                            .size(26.0)
                            .color(ACCENT),
                    );
                    ui.weak(language.text(Text::LinesUnit));
                });
                ui.separator();
                self.lines(ui);
                ui.separator();
                ui.label(
                    RichText::new(language.text(Text::RouteHeading))
                        .size(11.0)
                        .color(ACCENT)
                        .strong(),
                );
                ui.weak(language.text(Text::SelectStationHint));
                if let Some(id) = self.selected_station() {
                    let station = &self.metro.stations[id.0];
                    ui.label(RichText::new(&station.name).size(18.0).strong());
                    ui.colored_label(station.line.color(), language.line_name(station.line));
                    let open = station.access == super::StationAccess::Open;
                    if !open {
                        ui.weak(language.text(Text::StationClosed));
                    }
                    ui.add_enabled_ui(open, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button(language.text(Text::FromHere)).clicked() {
                                self.set_endpoint(0, id);
                            }
                            if ui.button(language.text(Text::ToHere)).clicked() {
                                self.set_endpoint(1, id);
                            }
                        })
                    });
                }
                for (index, placeholder) in [
                    language.text(Text::NoDeparture),
                    language.text(Text::NoDestination),
                ]
                .iter()
                .enumerate()
                {
                    ui.label(format!(
                        "{}  {}",
                        if index == 0 { "A" } else { "B" },
                        self.endpoints[index]
                            .map_or(*placeholder, |id| self.metro.stations[id.0].name.as_str())
                    ));
                }
                if let Some(route) = &self.route {
                    ui.label(
                        RichText::new(format!(
                            "{} {}",
                            route.minutes,
                            language.text(Text::Minutes)
                        ))
                        .size(28.0)
                        .color(ACCENT)
                        .strong(),
                    );
                    ui.label(format!(
                        "{}: {} · {}: {}",
                        language.text(Text::StationsCount),
                        route.stations.len(),
                        language.text(Text::TransfersCount),
                        route.transfers
                    ));
                    ui.weak(language.text(Text::TravelTimeHint));
                    egui::CollapsingHeader::new(language.text(Text::RouteStations))
                        .id_salt("route-stations")
                        .show(ui, |ui| {
                            for id in &route.stations {
                                let station = &self.metro.stations[id.0];
                                ui.colored_label(station.line.color(), &station.name);
                            }
                        });
                }
                if self.endpoints.iter().any(Option::is_some)
                    && ui.button(language.text(Text::ResetRoute)).clicked()
                {
                    self.clear_route();
                }
                ui.separator();
                ui.weak(language.city_description(self.city));
                ui.add_space(8.0);
                ui.hyperlink_to(
                    language.text(Text::AboutPautinka),
                    "https://github.com/hexqnt/pautinka#readme",
                );
            });
    }
}
