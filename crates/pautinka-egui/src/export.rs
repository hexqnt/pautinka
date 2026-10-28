//! Экспорт схем без окна и GPU. SVG сохраняет подписи контурами, PNG растеризует тот же SVG через resvg.

mod error;
mod options;
#[cfg(feature = "png")]
mod png;
mod svg;

use egui::{Context, FontDefinitions, Pos2, RawInput, Rect, Vec2};

use crate::{
    Canvas, Graph, InteractionOptions, Renderer, Style, geometry::valid_rect, render::RenderedGraph,
};

use options::Background;

pub use error::ExportError;
pub use options::ExportOptions;

/// Переиспользуемый контекст экспорта со своими шрифтами; состояние интерактивного холста не меняется.
///
/// Пользовательские renderer поддерживаются для векторных примитивов. Текстуры, GPU callbacks, градиентные штрихи и растровые цветные глифы возвращают явную ошибку.
pub struct Exporter {
    context: Context,
    fonts: FontDefinitions,
}

impl Exporter {
    /// Передайте те же определения шрифтов, что установлены в приложении, для согласованного оформления.
    pub fn new(fonts: FontDefinitions) -> Result<Self, ExportError> {
        if fonts.font_data.is_empty()
            || fonts
                .font_data
                .values()
                .any(|data| ttf_parser::Face::parse(data.bytes(), data.index).is_err())
            || fonts.families.values().any(|names| {
                names.is_empty() || names.iter().any(|name| !fonts.font_data.contains_key(name))
            })
            || !fonts.families.contains_key(&egui::FontFamily::Proportional)
        {
            return Err(ExportError::InvalidFonts);
        }
        let context = Context::default();
        context.set_fonts(fonts.clone());
        Ok(Self { context, fonts })
    }

    /// Возвращает самостоятельный SVG с векторными подписями, без ссылок на внешние файлы и шрифты.
    /// Автоматические границы включают нарисованные подписи и символы в пределах объявленных renderer полей.
    pub fn svg(
        &self,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        style: &Style,
        options: &ExportOptions,
    ) -> Result<String, ExportError> {
        let background = match options.background {
            Background::Style => Some(style.background),
            Background::Transparent => None,
            Background::Color(color) => Some(color),
        };
        let graph = RenderedGraph { graph, renderer };
        let graph_bounds = graph.bounds();
        if !valid_rect(graph_bounds) {
            return Err(ExportError::InvalidGeometry);
        }
        let mut canvas = Canvas {
            style: style.clone(),
            ..Canvas::default()
        };
        canvas.options.interaction = InteractionOptions::disabled();
        canvas.options.lod.enabled = false;
        canvas.style.label_min_zoom = 0.0;
        canvas.labels = options.labels;
        canvas.options.labels = options.labels.then_some(options.label_layout).flatten();
        canvas.show_loops = options.loops;
        if let Some(background) = background {
            canvas.style.background = background;
        }
        canvas.spatial.synchronize(&graph)?;
        let bounds = options
            .region
            .unwrap_or_else(|| graph_bounds.union(canvas.spatial.bounds()));
        let mut margin = 1.0_f32;
        if options.region.is_none() {
            for i in 0..graph.node_count() {
                margin = margin.max(graph.node_screen_margin(i));
            }
            for i in 0..graph.edge_count() {
                margin = margin.max(graph.edge_screen_margin(i));
            }
            for i in 0..graph.group_count() {
                margin = margin.max(graph.group_screen_margin(i));
            }
            margin += options.padding + canvas.crossing_margin();
        }
        let size = bounds.size() * options.scale + Vec2::splat(margin * 2.0);
        if !size.is_finite() || size.min_elem() <= 0.0 {
            return Err(ExportError::InvalidGeometry);
        }
        let viewport = Rect::from_min_size(Pos2::ZERO, size);
        canvas.camera = crate::Camera {
            center: bounds.min + bounds.size() * 0.5,
            zoom: options.scale,
            fit: false,
        };
        if !canvas.camera.center.is_finite()
            || !canvas.camera.to_screen(bounds.min, viewport).is_finite()
            || !canvas.camera.to_screen(bounds.max, viewport).is_finite()
        {
            return Err(ExportError::InvalidGeometry);
        }
        canvas.spatial.visible_groups.extend(0..graph.group_count());
        canvas.spatial.visible_nodes.extend(0..graph.node_count());
        canvas
            .spatial
            .visible_edges
            .extend((0..graph.edge_count()).filter(|&i| options.loops || !graph.edge(i).is_loop));
        if canvas.crossings_enabled() {
            canvas.crossings.prepare(&graph, &canvas.spatial);
        }
        let mut output = self.context.run_ui(
            RawInput {
                screen_rect: Some(viewport),
                ..RawInput::default()
            },
            |ui| {
                if graph.node_count() != 0 || graph.edge_count() != 0 || graph.group_count() != 0 {
                    canvas.paint(ui, &graph, renderer, viewport, true);
                }
            },
        );
        let shapes = std::mem::take(&mut output.shapes);
        output.drop_without_applying_deltas();
        let view_box = if options.region.is_some() {
            Rect::from_center_size(viewport.center(), bounds.size() * options.scale)
        } else {
            let painted = shapes.iter().fold(Rect::NOTHING, |bounds, shape| {
                bounds.union(
                    shape
                        .shape
                        .visual_bounding_rect()
                        .intersect(shape.clip_rect),
                )
            });
            if painted.is_finite() {
                painted.expand(options.padding)
            } else {
                viewport
            }
        };
        if !valid_rect(view_box) || view_box.size().min_elem() <= 0.0 {
            return Err(ExportError::InvalidGeometry);
        }
        svg::document(&shapes, view_box, background, &self.fonts)
    }

    /// PNG имеет `ceil(размер SVG × pixels_per_point)` пикселей по каждой оси.
    /// Ограничение в 64 миллиона пикселей проверяется до выделения RGBA-буфера.
    #[cfg(feature = "png")]
    pub fn png(
        &self,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        style: &Style,
        options: &ExportOptions,
        pixels_per_point: f32,
    ) -> Result<Vec<u8>, ExportError> {
        let scale = png::RasterScale::try_from(pixels_per_point)?;
        png::render(&self.svg(graph, renderer, style, options)?, scale)
    }
}
impl Default for Exporter {
    fn default() -> Self {
        Self::new(FontDefinitions::default()).expect("Bundled export fonts must be valid")
    }
}

#[cfg(test)]
mod tests;
