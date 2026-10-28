mod controls;
mod explorer;
mod header;
mod search;

use egui::{Color32, Pos2, Vec2};
use egui_frame_meter::{FrameHistory, FrameMeter};
use pautinka::egui::{
    Canvas, CanvasEvent, Graph, InteractionOptions, NavigatorCorner, NavigatorOptions, Selection,
};

use crate::i18n::{Language, Text};
use crate::metro::{City, Line, Metro, Route, StationAccess, StationId};
use crate::render;
use crate::render::{INK, MetroRenderer, PAPER};
use search::StationSearch;

struct Drag {
    origin: Pos2,
    before: Vec<(StationId, Pos2)>,
}

impl Drag {
    fn apply(&self, metro: &mut Metro, delta: Vec2) -> bool {
        let mut changed = false;
        for &(id, position) in &self.before {
            // При выходе за диапазон геометрии сохраняется последнее допустимое положение.
            changed |= metro.move_station(id, position + delta).unwrap_or(false);
        }
        changed
    }
}

pub struct Demo {
    city: City,
    metro: Metro,
    canvas: Canvas,
    search: StationSearch,
    line: Option<Line>,
    endpoints: [Option<StationId>; 2],
    route: Option<Route>,
    show_explorer: bool,
    explorer_collapsed: bool,
    configured: bool,
    drag: Option<Drag>,
    frame_time: FrameHistory,
}

impl Demo {
    pub fn language(&self) -> Language {
        self.metro.language
    }

    fn set_language(&mut self, language: Language) {
        self.metro.language = language;
        self.canvas.style.accessibility_label = language.map_label(self.city).into();
        #[cfg(target_arch = "wasm32")]
        language.apply_document(self.city);
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        if !self.configured {
            render::configure(ui.ctx());
            ui.set_style(ui.ctx().global_style());
            #[cfg(target_arch = "wasm32")]
            language.apply_document(self.city);
            self.configured = true;
        }
        let compact = ui.max_rect().width() < 780.0;
        self.header(ui, compact);
        egui::Panel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(17, 29, 40))
                    .inner_margin(10.0),
            )
            .show(ui, |ui| self.footer(ui));
        if !compact && !self.explorer_collapsed {
            egui::Panel::left("explorer")
                .exact_size(310.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(22, 35, 47))
                        .inner_margin(20.0),
                )
                .show(ui, |ui| self.explorer(ui));
        } else if compact && self.show_explorer {
            let mut open = self.show_explorer;
            egui::Window::new(language.text(Text::StationsRoute))
                .id(egui::Id::unique("stations-route"))
                .open(&mut open)
                .default_width(290.0)
                .max_height((ui.max_rect().height() - 100.0).max(120.0))
                .show(ui.ctx(), |ui| self.explorer(ui));
            self.show_explorer = open;
        }
        self.map(ui);
    }

    fn switch_city(&mut self, city: City) {
        let mut metro = Metro::for_city(city);
        metro.language = self.language();
        metro
            .set_geometry(self.metro.routing(), self.metro.corner_radius())
            .expect("Embedded metro geometry must support rounded corners");
        self.search = StationSearch::new(&metro);
        self.metro = metro;
        self.city = city;
        self.line = None;
        self.clear_route();
        self.drag = None;
        self.canvas.reset();
        self.canvas.style.accessibility_label = self.language().map_label(city).into();
        #[cfg(target_arch = "wasm32")]
        self.language().apply_document(city);
    }

    fn map(&mut self, ui: &mut egui::Ui) {
        let language = self.language();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(PAPER).inner_margin(14.0))
            .show(ui, |ui| {
                *ui.visuals_mut() = egui::Visuals::light();
                ui.visuals_mut().override_text_color = Some(INK);
                self.rendering_controls(ui);
                if self.canvas.labels()
                    && self.canvas.camera.zoom < self.canvas.style.label_min_zoom
                {
                    ui.weak(language.text(Text::ZoomLabelsHint));
                }
                let renderer = MetroRenderer {
                    metro: &self.metro,
                    line: self.line,
                    route: self.route.as_ref(),
                    endpoints: self.endpoints,
                };
                let response = self.canvas.show_with(ui, &self.metro, &renderer);
                self.frame_time.update(ui.ctx());
                let meter_size = FrameMeter::<240>::SIZE;
                let meter_anchor = if self.canvas.options.navigator.enabled
                    && self.canvas.options.navigator.corner
                        == pautinka::egui::NavigatorCorner::BottomRight
                {
                    egui::Align2::LEFT_BOTTOM
                } else {
                    egui::Align2::RIGHT_BOTTOM
                };
                let meter = meter_anchor.align_size_within_rect(
                    meter_size.min(response.viewport.size()),
                    response.viewport,
                );
                ui.place(meter, FrameMeter::new(&self.frame_time));
                self.apply_drag_events(ui.ctx());
            });
    }

    fn apply_drag_events(&mut self, context: &egui::Context) {
        for &event in self.canvas.events() {
            match event {
                CanvasEvent::NodeDragStarted { position, .. } => {
                    // Снимок выбранной группы создаётся один раз на жест для перемещения и отмены.
                    let before = self
                        .canvas
                        .selected_elements()
                        .filter_map(|element| match self.metro.resolve(element) {
                            Some(Selection::Node(index)) => {
                                Some((StationId(index), self.metro.stations[index].position))
                            }
                            _ => None,
                        })
                        .collect();
                    self.drag = Some(Drag {
                        origin: position,
                        before,
                    });
                }
                CanvasEvent::NodeDragged { position, .. }
                | CanvasEvent::NodeDragFinished { position, .. } => {
                    if let Some(drag) = &self.drag
                        && drag.apply(&mut self.metro, position - drag.origin)
                    {
                        context.request_repaint();
                    }
                    if matches!(event, CanvasEvent::NodeDragFinished { .. }) {
                        self.drag = None;
                    }
                }
                CanvasEvent::NodeDragCancelled { .. } => {
                    if let Some(drag) = self.drag.take()
                        && drag.apply(&mut self.metro, Vec2::ZERO)
                    {
                        context.request_repaint();
                    }
                }
                _ => {}
            }
        }
    }

    fn selected_station(&self) -> Option<StationId> {
        match self.canvas.selected() {
            Some(Selection::Node(index)) => Some(StationId(index)),
            _ => None,
        }
    }

    fn select(&mut self, id: StationId) {
        self.canvas.focus(Selection::Node(id.0), &self.metro);
    }

    fn clear_route(&mut self) {
        self.endpoints = [None; 2];
        self.route = None;
    }

    fn set_endpoint(&mut self, index: usize, id: StationId) {
        if self.metro.stations[id.0].access != StationAccess::Open {
            return;
        }
        self.endpoints[index] = Some(id);
        self.line = None;
        self.route = self.endpoints[0]
            .zip(self.endpoints[1])
            .and_then(|(from, to)| self.metro.route(from, to));
        if self.route.is_some() {
            self.canvas.camera.fit = true;
        }
    }
}

impl Demo {
    /// Создаёт демо с явно выбранным языком, независимо от локали системы.
    pub fn with_language(language: Language) -> Self {
        let city = City::default();
        let mut canvas = Canvas::default();
        canvas.style.background = PAPER;
        canvas.style.label_color = INK;
        canvas.style.label_min_zoom = 0.5;
        canvas.style.highlight = Color32::from_rgb(186, 112, 34);
        canvas.style.accessibility_label = language.map_label(city).into();
        canvas.options.interaction = InteractionOptions {
            snapping: true,
            ..InteractionOptions::default()
        };
        canvas.options.lod.enabled = false;
        canvas.options.navigator = NavigatorOptions {
            enabled: true,
            corner: NavigatorCorner::TopRight,
        };
        canvas.options.grid = Some({
            let mut grid = pautinka::egui::GridOptions::default();
            grid.visible = false;
            grid.minor_color = Color32::from_black_alpha(12);
            grid.major_color = Color32::from_black_alpha(30);
            grid
        });
        let mut metro = Metro::for_city(city);
        metro.language = language;
        let search = StationSearch::new(&metro);
        Self {
            city,
            metro,
            canvas,
            search,
            line: None,
            endpoints: [None; 2],
            route: None,
            show_explorer: false,
            explorer_collapsed: false,
            configured: false,
            drag: None,
            frame_time: FrameHistory::default(),
        }
    }
}

impl Default for Demo {
    fn default() -> Self {
        Self::with_language(Language::detect())
    }
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.show(ui);
        ui.ctx().request_repaint();
    }
}
