use pautinka_core::SpatialError;

use crate::{
    Canvas, Graph, GraphId, GraphRevision,
    render::{RenderedGraph, RendererKey},
};

use super::SpatialCache;

/// Индекс точного экземпляра и версии графа, подготовленный без доступа к UI.
pub struct PreparedGraph {
    cache: SpatialCache,
    crossings: crate::crossings::CrossingCache,
    renderer_key: RendererKey,
}

impl PreparedGraph {
    pub fn new(graph: &(impl Graph + ?Sized)) -> Result<Self, SpatialError> {
        Self::new_with(graph, &crate::DefaultRenderer)
    }
    pub fn new_with<R: crate::Renderer + ?Sized>(
        graph: &(impl Graph + ?Sized),
        renderer: &R,
    ) -> Result<Self, SpatialError> {
        let source = RenderedGraph { graph, renderer };
        let mut cache = SpatialCache::default();
        cache.synchronize(&source)?;
        let mut crossings = crate::crossings::CrossingCache::default();
        crossings.prepare(&source, &cache);
        Ok(Self {
            cache,
            crossings,
            renderer_key: RendererKey::new(renderer),
        })
    }
    pub fn graph_id(&self) -> GraphId {
        self.cache.key.expect("Prepared graph has a version").0
    }
    pub fn revision(&self) -> GraphRevision {
        self.cache.key.expect("Prepared graph has a version").1
    }
}

impl Canvas {
    /// Принимает подготовленный индекс только для того же экземпляра и версии.
    /// Камера и выбор остаются состоянием текущего холста.
    pub fn install_prepared(
        &mut self,
        prepared: PreparedGraph,
        graph: &(impl Graph + ?Sized),
    ) -> bool {
        self.install_prepared_with(prepared, graph, &crate::DefaultRenderer)
    }
    pub fn install_prepared_with<R: crate::Renderer + ?Sized>(
        &mut self,
        prepared: PreparedGraph,
        graph: &(impl Graph + ?Sized),
        renderer: &R,
    ) -> bool {
        if !prepared.cache.ready(graph) {
            return false;
        }
        if prepared.renderer_key != RendererKey::new(renderer) {
            return false;
        }
        self.renderer_key = Some(prepared.renderer_key);
        self.spatial = prepared.cache;
        self.crossings = prepared.crossings;
        self.hit_cache = None;
        self.synchronize(graph);
        true
    }
    /// Подготовку большого графа можно выполнить в рабочем потоке до первого показа.
    pub fn prepare(&mut self, graph: &(impl Graph + ?Sized)) -> Result<(), SpatialError> {
        self.prepare_with(graph, &crate::DefaultRenderer)
    }
    pub fn prepare_with(
        &mut self,
        graph: &(impl Graph + ?Sized),
        renderer: &(impl crate::Renderer + ?Sized),
    ) -> Result<(), SpatialError> {
        self.use_renderer(renderer);
        let source = RenderedGraph { graph, renderer };
        self.synchronize(&source);
        self.spatial.synchronize(&source)?;
        if self.crossings_enabled() {
            self.crossings.prepare(&source, &self.spatial);
        }
        Ok(())
    }
}
