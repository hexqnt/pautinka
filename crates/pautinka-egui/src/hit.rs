use egui::{Pos2, Rect, Ui};

use crate::{
    Canvas, DefaultRenderer, ElementId, Graph, GraphId, GraphRevision, HitPoint, Renderer,
    Selection,
    render::{RenderedGraph, RendererKey},
};

#[derive(Clone, Copy, PartialEq)]
pub(super) struct HitKey {
    graph: GraphId,
    point: HitPoint,
    zoom: f32,
    revision: GraphRevision,
    loops: bool,
    crossings: crate::CrossingOptions,
}

impl Canvas {
    pub fn hovered(&self) -> Option<ElementId> {
        self.hovered
    }
    pub fn context_target(&self) -> Option<ElementId> {
        self.context_target
    }

    pub(super) fn update_hover(
        &mut self,
        ui: &Ui,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        viewport: Rect,
    ) {
        let pointer = ui
            .input(|input| input.pointer.hover_pos())
            .filter(|_| ui.rect_contains_pointer(viewport));
        if let Some(point) =
            pointer.and_then(|pointer| self.camera.to_world_precise(pointer, viewport))
        {
            let key = HitKey {
                graph: graph.id(),
                point,
                zoom: self.camera.zoom,
                revision: graph.revision(),
                loops: self.show_loops,
                crossings: self.style.crossings,
            };
            if self.crossing_margin() > 0.0
                || self
                    .hit_cache
                    .as_ref()
                    .is_none_or(|(previous, _)| *previous != key)
            {
                self.hit_cache = Some((key, self.pick_source(point, graph, renderer, true)));
            }
            self.hovered = self
                .hit_cache
                .and_then(|(_, hit)| hit)
                .and_then(|hit| graph.identity(hit));
        } else {
            self.hovered = None;
            self.hit_cache = None;
        }
    }

    /// Проверка выбора использует те же пути и размеры, что и рисование.
    pub fn pick(&self, point: Pos2, graph: &(impl Graph + ?Sized)) -> Option<Selection> {
        self.pick_with(point, graph, &DefaultRenderer)
    }

    pub fn pick_with<R: Renderer + ?Sized>(
        &self,
        point: Pos2,
        graph: &(impl Graph + ?Sized),
        renderer: &R,
    ) -> Option<Selection> {
        self.pick_precise_with(HitPoint::from_world(point)?, graph, renderer)
    }

    pub fn pick_precise(
        &self,
        point: HitPoint,
        graph: &(impl Graph + ?Sized),
    ) -> Option<Selection> {
        self.pick_precise_with(point, graph, &DefaultRenderer)
    }

    pub fn pick_precise_with<R: Renderer + ?Sized>(
        &self,
        point: HitPoint,
        graph: &(impl Graph + ?Sized),
        renderer: &R,
    ) -> Option<Selection> {
        let source = RenderedGraph { graph, renderer };
        let indexed = self.renderer_key == Some(RendererKey::new(renderer));
        self.pick_source(point, &source, renderer, indexed)
    }

    pub(super) fn pick_source(
        &self,
        point: HitPoint,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        indexed: bool,
    ) -> Option<Selection> {
        let original = self.pick_base(point, graph, indexed);
        if matches!(original, Some(Selection::Node(_))) {
            return original;
        }
        let bridge_margin = self.crossing_margin();
        if bridge_margin == 0.0 {
            return original;
        }
        let mut local_spatial = crate::spatial::SpatialCache::default();
        let mut local_crossings = crate::crossings::CrossingCache::default();
        let (spatial, cache) =
            if indexed && self.spatial.ready(graph) && self.crossings.ready(graph) {
                (&self.spatial, &self.crossings)
            } else {
                if local_spatial.synchronize(graph).is_err() {
                    return original;
                }
                local_crossings.prepare(graph, &local_spatial);
                (&local_spatial, &local_crossings)
            };
        let width = |index| self.crossing_width(graph, renderer, index);
        let view = crate::crossings::CrossingView {
            graph,
            defaults: self.style.crossings,
            overrides: &self.crossing_overrides,
            zoom: self.camera.zoom,
            width: &width,
        };
        let original_distance = original.and_then(|selection| {
            let Selection::Edge(index) = selection else {
                return None;
            };
            graph
                .edge(index)
                .path
                .hit_distance_sq_precise(point, graph.edge_pick_radius(index) / self.camera.zoom)
        });
        let mut best = None;
        spatial.bridge_candidates(point, self.camera.zoom, bridge_margin, |index, segment| {
            let edge = graph.edge(index);
            if !self.show_loops && edge.is_loop {
                return;
            }
            let radius_sq = f64::from(graph.edge_pick_radius(index) / self.camera.zoom).powi(2);
            cache.bridges(index, Some(segment), &view, |points| {
                let Some(distance) = points
                    .windows(2)
                    .map(|pair| point.segment_distance_sq(pair[0], pair[1]))
                    .min_by(f64::total_cmp)
                else {
                    return;
                };
                if distance <= radius_sq
                    && best.is_none_or(|(_, id, previous)| {
                        distance < previous || (distance == previous && edge.id < id)
                    })
                {
                    best = Some((index, edge.id, distance));
                }
            });
        });
        // Видимая дуга перекрывает нижнюю линию; допускаем субпиксельное округление камеры только при сравнении с исходным путём.
        let tolerance_sq = (0.01 / f64::from(self.camera.zoom)).powi(2);
        best.filter(|&(_, _, distance)| {
            original_distance.is_none_or(|previous| distance <= previous + tolerance_sq)
        })
        .map(|(index, _, _)| Selection::Edge(index))
        .or(original)
    }

    fn pick_base(
        &self,
        point: HitPoint,
        graph: &(impl Graph + ?Sized),
        indexed: bool,
    ) -> Option<Selection> {
        if indexed && self.spatial.ready(graph) {
            return self
                .spatial
                .pick(point, self.camera.zoom, self.show_loops, graph);
        }
        for index in (0..graph.node_count()).rev() {
            if graph.node_hit_test(index, point, self.camera.zoom) {
                return Some(Selection::Node(index));
            }
        }
        self.visible_edges(graph)
            .filter_map(|(index, edge)| {
                let radius = graph.edge_pick_radius(index) / self.camera.zoom;
                if !edge
                    .path
                    .bounds()
                    .expand(radius + point.rounding_error())
                    .contains(point.rounded())
                {
                    return None;
                }
                edge.path
                    .hit_distance_sq_precise(point, radius)
                    .map(|distance| (index, edge.id, distance))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.1.cmp(&b.1)))
            .map(|(index, _, _)| Selection::Edge(index))
            .or_else(|| {
                (0..graph.group_count())
                    .rev()
                    .find(|&index| graph.group(index).hit_test(point, self.camera.zoom))
                    .map(Selection::Group)
            })
    }
}
