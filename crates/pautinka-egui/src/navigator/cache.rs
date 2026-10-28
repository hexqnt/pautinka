use egui::{Color32, Context, Painter, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

use crate::{Camera, Graph, GraphId, GraphRevision};

use super::{Navigator, graph_shapes, paint_shapes};

mod raster;

#[derive(PartialEq)]
struct Key {
    context: Context,
    graph: GraphId,
    revision: GraphRevision,
    counts: [usize; 3],
    camera: Camera,
    size: Vec2,
    loops: bool,
    pixels_per_point: f32,
    anti_alias: bool,
}

#[derive(Default)]
pub(crate) struct NavigatorCache {
    key: Option<Key>,
    changed_frame: u64,
    texture: Option<TextureHandle>,
}

impl NavigatorCache {
    pub fn paint(
        &mut self,
        painter: &Painter,
        graph: &(impl Graph + ?Sized),
        navigator: &Navigator,
        loops: bool,
    ) {
        let context = painter.ctx();
        let frame = context.cumulative_frame_nr();
        let key = Key {
            context: context.clone(),
            graph: graph.id(),
            revision: graph.revision(),
            counts: [graph.node_count(), graph.edge_count(), graph.group_count()],
            camera: navigator.camera,
            size: navigator.thumbnail.size(),
            loops,
            pixels_per_point: context.pixels_per_point(),
            anti_alias: context.tessellation_options(|options| options.feathering),
        };
        if self.key.as_ref() != Some(&key) {
            self.key = Some(key);
            self.changed_frame = frame;
            self.texture = None;
            context.request_repaint();
        } else if self.texture.is_none() && frame > self.changed_frame {
            // Растеризуем только после неподвижного кадра; анимация сохраняет векторный путь без лишних загрузок текстур.
            self.texture = raster::image(
                graph,
                navigator,
                loops,
                key.pixels_per_point,
                key.anti_alias,
            )
            .map(|image| context.load_texture("pautinka-navigator", image, TextureOptions::LINEAR));
        }
        if let Some(texture) = &self.texture {
            painter.image(
                texture.id(),
                navigator.thumbnail,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            paint_shapes(painter, graph_shapes(graph, navigator, loops));
        }
    }
}
