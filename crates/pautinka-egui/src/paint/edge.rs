use egui::{
    Align2, Color32, FontId, Pos2, Rect, Shape,
    epaint::{CubicBezierShape, QuadraticBezierShape},
};

use crate::{Canvas, DetailLevel, Edge, EdgeStyle, ElementId, RenderContext, geometry::Curve};

use super::pattern::Pattern;

const BRIDGE_BACKGROUND_PADDING: f32 = 3.0;

impl Canvas {
    pub(super) fn paint_edges(
        &self,
        graph: &(impl crate::Graph + ?Sized),
        renderer: &(impl crate::Renderer + ?Sized),
        context: &RenderContext<'_>,
    ) {
        let viewport = context.viewport;
        let painter = context.painter;
        let feathering = painter
            .ctx()
            .tessellation_options(|options| options.feathering_size_in_pixels.max(0.0));
        let feather_margin =
            std::f32::consts::SQRT_2 * (1.0 + feathering) / painter.ctx().pixels_per_point();
        let width = |index| self.crossing_width(graph, renderer, index);
        let crossings = crate::crossings::CrossingView {
            graph,
            defaults: self.style.crossings,
            overrides: &self.crossing_overrides,
            zoom: self.camera.zoom,
            width: &width,
        };
        for &index in &self.spatial.visible_edges {
            let edge = graph.edge(index);
            let state = self.element_state(ElementId::Edge(edge.id));
            let style = renderer.edge_style(index, edge, state, &self.style);
            let margin = graph
                .edge_screen_margin(index)
                .max(style.arrow_size.length())
                .max(style.outer_width())
                + crossings.options(index).margin()
                + feather_margin;
            let clip = viewport.expand(margin);
            let bounds = self.camera.rect_to_world(clip, viewport);
            if !bounds.intersects(edge.path.bounds()) {
                continue;
            }
            let full = self.detail == DetailLevel::Full || state.emphasized();
            self.paint_edge(context, index, edge, style, bounds, clip, full, &crossings);
            if full
                && self.labels
                && let Some(label) = renderer.edge_label(index)
            {
                painter.text(
                    self.camera.to_screen(edge.path.midpoint(), viewport),
                    Align2::CENTER_BOTTOM,
                    label,
                    FontId::proportional(11.0),
                    self.style.label_color,
                );
            }
            renderer.paint_edge_overlay(context, index, edge, state);
        }
        if self.crossing_margin() > 0.0 {
            for &index in &self.spatial.visible_edges {
                if crossings.options(index).mode != crate::CrossingMode::Bridge {
                    continue;
                }
                let edge = graph.edge(index);
                let state = self.element_state(ElementId::Edge(edge.id));
                let style = renderer.edge_style(index, edge, state, &self.style);
                let margin = graph.edge_screen_margin(index).max(style.outer_width())
                    + crossings.options(index).margin()
                    + feather_margin
                    + BRIDGE_BACKGROUND_PADDING;
                let clip = viewport.expand(margin);
                self.paint_bridges(
                    context,
                    index,
                    style,
                    self.camera.rect_to_world(clip, viewport),
                    &crossings,
                );
            }
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn paint_edge<G: crate::Graph + ?Sized>(
        &self,
        context: &RenderContext<'_>,
        index: usize,
        edge: Edge<'_>,
        style: EdgeStyle,
        bounds: Rect,
        arrow_bounds: Rect,
        full: bool,
        crossings: &crate::crossings::CrossingView<'_, G>,
    ) {
        let camera = context.camera;
        let viewport = context.viewport;
        let painter = context.painter;
        let mut arrows = smallvec::SmallVec::<[[Pos2; 2]; 4]>::new();
        if full {
            for &reverse in edge.direction.arrows() {
                let sample = edge.path.arrow_sample(reverse);
                let tip = camera.to_screen(sample.position, viewport);
                if !arrow_bounds.contains(tip) {
                    continue;
                }
                let back = tip - sample.tangent * style.arrow_size.x;
                let normal = sample.tangent.rot90() * style.arrow_size.y;
                arrows.extend([[back + normal, tip], [back - normal, tip]]);
            }
        }
        let pattern = Pattern::new(style.pattern, style.stroke.width);
        let mut flattened = smallvec::SmallVec::<[Vec<Pos2>; 2]>::new();
        for stroke in style.strokes() {
            let mut section = 0;
            self.crossings.curves(index, crossings, |curve| {
                let line = |points: &[Pos2]| pattern.paint(painter, points, stroke);
                let shape = match curve {
                    Curve::Polyline(points) => {
                        if !bounds.intersects(Rect::from_points(points)) {
                            return;
                        }
                        let points: smallvec::SmallVec<[_; 8]> = points
                            .iter()
                            .map(|&p| camera.to_screen(p, viewport))
                            .collect();
                        line(&points);
                        None
                    }
                    Curve::Cubic(points) => {
                        Some(Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                            points.map(|p| camera.to_screen(p, viewport)),
                            false,
                            Color32::TRANSPARENT,
                            stroke,
                        )))
                    }
                    Curve::Quadratic(points) => Some(Shape::QuadraticBezier(
                        QuadraticBezierShape::from_points_stroke(
                            points.map(|p| camera.to_screen(p, viewport)),
                            false,
                            Color32::TRANSPARENT,
                            stroke,
                        ),
                    )),
                };
                match shape {
                    Some(Shape::CubicBezier(curve)) if !pattern.is_solid() => {
                        if flattened.len() == section {
                            flattened.push(curve.flatten(Some(0.25)));
                        }
                        line(&flattened[section]);
                        section += 1;
                    }
                    Some(Shape::QuadraticBezier(curve)) if !pattern.is_solid() => {
                        if flattened.len() == section {
                            flattened.push(curve.flatten(Some(0.25)));
                        }
                        line(&flattened[section]);
                        section += 1;
                    }
                    Some(shape) => {
                        painter.add(shape);
                    }
                    None => {}
                }
            });
            if !arrows.is_empty() {
                painter.extend(
                    arrows
                        .iter()
                        .map(|&points| Shape::line_segment(points, stroke)),
                );
            }
        }
    }

    fn paint_bridges<G: crate::Graph + ?Sized>(
        &self,
        context: &RenderContext<'_>,
        index: usize,
        style: EdgeStyle,
        bounds: Rect,
        crossings: &crate::crossings::CrossingView<'_, G>,
    ) {
        let pattern = Pattern::new(style.pattern, style.stroke.width);
        self.crossings.bridges(index, None, crossings, |points| {
            if !bounds.intersects(Rect::from_points(points)) {
                return;
            }
            let points: smallvec::SmallVec<[_; 32]> = points
                .iter()
                .map(|&p| context.camera.to_screen(p, context.viewport))
                .collect();
            // Подложка скрывает нижнее ребро; её рисуем до halo и основного штриха.
            context.painter.add(Shape::line(
                points.to_vec(),
                egui::Stroke::new(
                    style.outer_width() + BRIDGE_BACKGROUND_PADDING,
                    self.style.background,
                ),
            ));
            for stroke in style.strokes() {
                pattern.paint(context.painter, &points, stroke);
            }
        });
    }
}
