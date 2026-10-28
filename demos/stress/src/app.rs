mod controls;
mod growth;
mod render;
mod run;
mod statistics;
#[cfg(target_arch = "wasm32")]
mod visibility;

use std::time::Duration;

use egui::{Rect, Ui};
use pautinka::egui::{Canvas, GridOptions};
use web_time::Instant;

use crate::{
    Scenario,
    metrics::{CpuSample, Metrics},
    scenario::Pattern,
    scene::{Load, Motion, NodeCount, Scene},
};
use controls::Controls;
use growth::Growth;
use run::Run;

const MAX_ANIMATION_STEP: Duration = Duration::from_millis(50);

pub struct Demo {
    scene: Scene,
    initial_nodes: NodeCount,
    canvas: Canvas,
    seed: u32,
    motion: Motion,
    running: bool,
    auto_fit: bool,
    growth: Growth,
    run: Run,
    pattern: Pattern,
    configured: bool,
    metrics: Metrics,
    hud_text: [String; 7],
    visible: (usize, usize),
    viewport: Rect,
    #[cfg(target_arch = "wasm32")]
    visibility: visibility::Visibility,
}

impl Default for Demo {
    fn default() -> Self {
        Self::with_scenario(Scenario::default())
    }
}

impl Demo {
    pub fn new(seed: u32, load: Load) -> Self {
        Self::with_scenario(Scenario {
            seed,
            load,
            ..Scenario::default()
        })
    }

    /// Создаёт демо с разобранными настройками и условиями завершения.
    pub fn with_scenario(scenario: Scenario) -> Self {
        let mut canvas = Canvas::default();
        canvas.set_labels(scenario.view.labels);
        canvas.style.accessibility_label = "Animated graph".into();
        canvas.style.label_min_zoom = 0.6;
        canvas.style.crossings.mode = scenario.crossings;
        canvas.options.navigator.enabled = scenario.view.minimap;
        canvas.options.lod.enabled = scenario.view.lod;
        canvas.options.grid = scenario.view.grid.then(GridOptions::default);
        Self {
            scene: Scene::with_options(
                scenario.seed,
                scenario.load,
                scenario.style,
                scenario.topology,
            ),
            initial_nodes: scenario.load.node_count(),
            canvas,
            seed: scenario.seed,
            motion: scenario.motion,
            running: scenario.animate,
            auto_fit: scenario.view.auto_fit,
            growth: Growth::new(scenario.growth),
            run: Run::new(scenario.limits),
            pattern: scenario.pattern,
            configured: false,
            metrics: Metrics::default(),
            hud_text: [const { String::new() }; 7],
            visible: (0, 0),
            viewport: Rect::NOTHING,
            #[cfg(target_arch = "wasm32")]
            visibility: visibility::Visibility::default(),
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn auto_grow(&self) -> bool {
        self.growth.enabled
    }
    pub fn sample_count(&self) -> usize {
        self.metrics.summary.count
    }

    /// Отключение роста оставляет фиксированную нагрузку для воспроизводимых CPU-сценариев.
    pub fn set_auto_grow(&mut self, enabled: bool) {
        self.growth.enabled = enabled;
        self.growth.reset();
    }

    pub fn show(&mut self, ui: &mut Ui) {
        if !self.configured {
            egui_extras::install_image_loaders(ui.ctx());
            ui.ctx().set_visuals(egui::Visuals::dark());
            ui.set_style(ui.ctx().global_style());
            self.configured = true;
        }
        #[cfg(target_arch = "wasm32")]
        if self.visibility.take_resumed() {
            self.metrics.resume();
        }

        let now = ui.input(|input| input.time);
        let elapsed = self.metrics.begin_frame(now);
        let controls = self.controls(ui);
        if controls.restart {
            self.run.reset();
        }
        let mut cpu = self.update_scene(&controls, elapsed);
        if controls.changed || cpu.build.is_some() {
            self.metrics.clear();
        }
        let refresh_statistics = self.metrics.refresh(now);
        if refresh_statistics {
            self.update_visible_counts();
        }

        self.map(ui, &mut cpu, refresh_statistics);
        self.metrics.record_cpu(cpu);
        if self.run.finished(now, self.scene.load()) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // Даже при паузе нужны интервалы реальных кадров, а не только кадры от ввода.
        ui.ctx().request_repaint();
    }

    fn update_scene(&mut self, controls: &Controls, elapsed: Duration) -> CpuSample {
        let start = Instant::now();
        let mut build = None;
        if controls.restart {
            self.scene = Scene::with_options(
                self.seed,
                controls.load.with_node_count(self.initial_nodes),
                controls.style,
                controls.topology,
            );
            build = Some(start.elapsed());
            self.canvas.reset();
            self.growth.reset();
        } else {
            self.scene.set_style(controls.style);
            let mut load = controls.load;
            if load == self.scene.load()
                && !self.scene.dragging()
                && let Some(next) = self.growth.advance(elapsed, load)
            {
                load = next;
            }
            if load != self.scene.load() || controls.topology != self.scene.topology() {
                let start = Instant::now();
                self.scene.reconfigure(load, controls.topology);
                build = Some(start.elapsed());
                self.growth.reset();
            }
        }
        if self.running {
            self.scene
                .advance(elapsed.min(MAX_ANIMATION_STEP), self.motion);
        }
        CpuSample {
            model: start.elapsed(),
            build,
            ..CpuSample::default()
        }
    }

    fn map(&mut self, ui: &mut Ui, cpu: &mut CpuSample, refresh_statistics: bool) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let start = Instant::now();
                if self.auto_fit && !self.scene.dragging() {
                    self.canvas.camera.fit = true;
                }
                let response = self.canvas.show_with(ui, &self.scene, &self.pattern);
                self.viewport = response.viewport;
                if response.response.clicked_by(egui::PointerButton::Middle) {
                    self.canvas.camera.fit = true;
                }
                cpu.widget = start.elapsed();
                let start = Instant::now();
                for &event in self.canvas.events() {
                    self.scene.handle_event(event);
                }
                cpu.model += start.elapsed();
                if refresh_statistics {
                    self.refresh_statistics();
                }
                self.statistics(ui);
            });
    }

    fn update_visible_counts(&mut self) {
        if !self.viewport.is_positive() {
            return;
        }
        let bounds = Rect::from_min_max(
            self.canvas
                .camera
                .to_world(self.viewport.min, self.viewport),
            self.canvas
                .camera
                .to_world(self.viewport.max, self.viewport),
        );
        self.visible = self.scene.visible_counts(bounds);
    }
}

impl eframe::App for Demo {
    fn ui(&mut self, ui: &mut Ui, _: &mut eframe::Frame) {
        self.show(ui);
    }
}
