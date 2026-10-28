use crate::{Canvas, EdgeId, ElementId, Graph, Renderer};

use super::{CrossingMode, CrossingOptions, EdgeCrossings};

impl Canvas {
    pub fn edge_crossings(&self, id: EdgeId) -> Option<EdgeCrossings> {
        self.crossing_overrides.get(&id).copied()
    }

    /// None снимает переопределение. Изменения оформления не перестраивают поиск пересечений.
    pub fn set_edge_crossings(&mut self, id: EdgeId, settings: Option<EdgeCrossings>) {
        if self.edge_crossings(id) == settings {
            return;
        }
        if let Some(settings) = settings {
            self.crossing_overrides.insert(id, settings);
        } else {
            self.crossing_overrides.remove(&id);
        }
        self.hit_cache = None;
    }

    pub(crate) fn crossings_enabled(&self) -> bool {
        self.style.crossings.mode != CrossingMode::None
            || self.crossing_overrides.values().any(|settings| {
                settings
                    .options
                    .is_some_and(|options| options.mode != CrossingMode::None)
            })
    }

    pub(crate) fn crossing_margin(&self) -> f32 {
        self.crossing_overrides
            .values()
            .filter_map(|s| s.options)
            .map(CrossingOptions::margin)
            .fold(self.style.crossings.margin(), f32::max)
    }

    pub(crate) fn crossing_width(
        &self,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl Renderer + ?Sized),
        index: usize,
    ) -> f32 {
        let edge = graph.edge(index);
        let style = renderer.edge_style(
            index,
            edge,
            self.element_state(ElementId::Edge(edge.id)),
            &self.style,
        );
        style.outer_width()
    }
}
