use std::time::{Duration, Instant};

use egui::{Context, Event, RawInput, Rect, Ui};

/// Общий CPU-harness с фиксированным шагом 1/60 с, без GPU и дерева AccessKit.
#[derive(Default)]
pub struct CpuUiHarness {
    context: Context,
    time: f64,
}

#[derive(Clone, Copy)]
pub struct FrameSample {
    pub ui: Duration,
    pub tessellation: Duration,
    pub shapes: usize,
    pub vertices: usize,
}

impl CpuUiHarness {
    pub fn frame(
        &mut self,
        viewport: Rect,
        events: Vec<Event>,
        show: impl FnMut(&mut Ui),
    ) -> FrameSample {
        self.time += 1.0 / 60.0;
        let input = RawInput {
            screen_rect: Some(viewport),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let start = Instant::now();
        let mut output = self.context.run_ui(input, show);
        let ui = start.elapsed();
        // Текстуры не отправляются на GPU; удерживать обновления между кадрами не нужно.
        output.textures_delta.clear();
        let shapes = output.shapes.len();
        let start = Instant::now();
        let meshes = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let tessellation = start.elapsed();
        let vertices = meshes
            .iter()
            .map(|primitive| match &primitive.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.len(),
                egui::epaint::Primitive::Callback(_) => 0,
            })
            .sum();
        FrameSample {
            ui,
            tessellation,
            shapes,
            vertices,
        }
    }
}
