//! Настраиваемая раскладка подписей узлов в экранных координатах; включается через `CanvasOptions::labels`.

mod grid;
mod layout;
mod options;

use std::sync::Arc;

use egui::{Color32, Galley, Painter, Rect};

use crate::{Canvas, Graph, GraphId, Node, NodeId, NodeVisual, RenderContext, Renderer};

use layout::{Layout, Request};

pub use options::{
    LabelEmphasis, LabelLayoutOptions, LabelPlacement, LabelPosition, LabelPositions,
    LabelVisibility, LabelWeights, NodeLabelOptions,
};

/// Результат последнего показа холста; прямоугольник включает видимые границы глифов.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedNodeLabel {
    pub id: NodeId,
    pub index: usize,
    pub rect: Rect,
    pub position: LabelPosition,
    pub forced: bool,
    pub conflicted: bool,
    pub(super) source: usize,
}

/// Измеренный текст используется и для раскладки, и для рисования без повторного измерения.
pub struct NodeLabelVisual<'a> {
    pub placement: &'a PlacedNodeLabel,
    pub galley: &'a Arc<Galley>,
    pub text_bounds: Rect,
    pub color: Color32,
    pub background: Option<Color32>,
    pub background_padding: f32,
}

struct LabelPaint {
    galley: Arc<Galley>,
    bounds: Rect,
    color: Color32,
}

pub(crate) struct LabelCache {
    graph: GraphId,
    layout: Layout,
    paints: Vec<LabelPaint>,
}

/// Сбор использует уже вычисленные геометрию, состояние и стиль текущего прохода рисования.
pub(super) struct LabelFrame {
    cache: Box<LabelCache>,
    options: LabelLayoutOptions,
}

impl LabelFrame {
    pub fn push(
        &mut self,
        renderer: &(impl Renderer + ?Sized),
        painter: &Painter,
        node: Node<'_>,
        visual: &NodeVisual,
        ordinary_labels: bool,
    ) {
        let options = self.options;
        // Узлы без подписи также остаются препятствиями.
        if options.weights.nodes > 0.0 {
            self.cache.layout.nodes.push(visual.rect);
        }
        if node.label.is_empty() {
            return;
        }
        let label = renderer.node_label_options(visual.index, node);
        if label.visibility == LabelVisibility::Hidden {
            return;
        }
        let importance = options.emphasis.importance(visual.state);
        let forced = label.visibility == LabelVisibility::Always || importance > 0;
        if !forced && (!ordinary_labels || options.max_labels == 0) {
            return;
        }
        let style = &visual.style;
        // egui принимает String; одно измерение переиспользуется при рисовании и учитывает замену шрифтов.
        let galley = painter.layout_no_wrap(
            node.label.to_owned(),
            style.label_font.clone(),
            style.label_color,
        );
        let bounds = galley.rect.union(galley.mesh_bounds);
        if !bounds.is_finite() || !bounds.is_positive() {
            return;
        }
        self.cache.layout.requests.push(Request {
            id: node.id,
            index: visual.index,
            node: visual.rect,
            size: bounds.size(),
            options: label,
            forced,
            importance,
        });
        self.cache.paints.push(LabelPaint {
            galley,
            bounds,
            color: style.label_color,
        });
    }

    pub fn finish(mut self, viewport: Rect) -> Box<LabelCache> {
        self.cache.layout.solve(viewport, self.options);
        self.cache
    }
}
impl Canvas {
    /// Показанные умные подписи последнего кадра; пусто при выключенной автоматике.
    pub fn node_labels(&self) -> &[PlacedNodeLabel] {
        if self.options.labels.is_none() {
            return &[];
        }
        self.label_cache
            .as_ref()
            .map_or(&[], |cache| &cache.layout.placed)
    }

    /// Число пересчётов размещения; неподвижные кадры используют прежнее решение.
    pub fn label_layout_solves(&self) -> u64 {
        self.label_cache
            .as_ref()
            .map_or(0, |cache| cache.layout.solves)
    }

    pub(super) fn begin_labels(&mut self, graph: GraphId) -> Option<LabelFrame> {
        let Some(options) = self.options.labels else {
            self.label_cache = None;
            return None;
        };
        let mut cache = match self.label_cache.take() {
            Some(cache) if cache.graph == graph => cache,
            _ => Box::new(LabelCache {
                graph,
                layout: Layout::default(),
                paints: Vec::new(),
            }),
        };
        cache.layout.requests.clear();
        cache.layout.nodes.clear();
        cache.paints.clear();
        Some(LabelFrame { cache, options })
    }

    pub(super) fn paint_labels(
        &self,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        painter: &Painter,
        viewport: Rect,
    ) {
        let Some(options) = self.options.labels else {
            return;
        };
        let Some(cache) = &self.label_cache else {
            return;
        };
        let context = RenderContext {
            painter,
            camera: self.camera,
            viewport,
            style: &self.style,
            detail: self.detail,
        };
        // Обязательные подписи размещаются первыми, но рисуются последними поверх обычных.
        for placement in cache.layout.placed.iter().rev() {
            let paint = &cache.paints[placement.source];
            renderer.paint_node_label(
                &context,
                graph.node(placement.index),
                &NodeLabelVisual {
                    placement,
                    galley: &paint.galley,
                    text_bounds: paint.bounds,
                    color: paint.color,
                    background: if placement.forced && placement.conflicted {
                        options.conflict_background
                    } else {
                        None
                    },
                    background_padding: options.background_padding,
                },
            );
        }
    }
}

pub fn paint_node_label(context: &RenderContext<'_>, visual: &NodeLabelVisual<'_>) {
    if let Some(background) = visual.background {
        context.painter.rect_filled(
            visual.placement.rect.expand(visual.background_padding),
            2.0,
            background,
        );
    }
    context.painter.galley(
        visual.placement.rect.min - visual.text_bounds.min.to_vec2(),
        Arc::clone(visual.galley),
        visual.color,
    );
}

#[cfg(test)]
mod tests;
