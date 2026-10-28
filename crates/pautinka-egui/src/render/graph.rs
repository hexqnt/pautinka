use egui::{Rect, Ui};

use crate::{
    ChangeCoverage, Edge, ElementId, Graph, GraphId, GraphRevision, HitPoint, Node,
    NodeInteraction, NodeShape, Port, Selection,
};

use super::{NodeGeometry, Renderer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RendererKey {
    type_name: &'static str,
    revision: u64,
}

impl RendererKey {
    pub fn new<R: Renderer + ?Sized>(renderer: &R) -> Self {
        Self {
            type_name: std::any::type_name::<R>(),
            revision: renderer.revision(),
        }
    }
}

pub(crate) struct RenderedGraph<'a, G: ?Sized, R: ?Sized> {
    pub graph: &'a G,
    pub renderer: &'a R,
}

impl<G: Graph + ?Sized, R: Renderer + ?Sized> Graph for RenderedGraph<'_, G, R> {
    fn group_count(&self) -> usize {
        self.graph.group_count()
    }
    fn group(&self, index: usize) -> crate::Group<'_> {
        self.graph.group(index)
    }
    fn group_interaction(&self, index: usize) -> crate::GroupInteraction {
        self.graph.group_interaction(index)
    }
    fn group_screen_margin(&self, index: usize) -> f32 {
        self.graph
            .group_screen_margin(index)
            .max(self.renderer.group_screen_margin(index, self.group(index)))
    }
    fn group_tooltip(&self, ui: &mut Ui, index: usize) {
        self.graph.group_tooltip(ui, index);
    }
    fn id(&self) -> GraphId {
        self.graph.id()
    }
    fn revision(&self) -> GraphRevision {
        self.graph.revision()
    }
    fn node_count(&self) -> usize {
        self.graph.node_count()
    }
    fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
    fn node(&self, index: usize) -> Node<'_> {
        self.graph.node(index)
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        self.graph.edge(index)
    }
    fn bounds(&self) -> Rect {
        self.graph.bounds()
    }
    fn node_shape(&self, index: usize) -> Option<NodeShape> {
        self.graph.node_shape(index)
    }
    fn node_geometry(&self, index: usize) -> NodeGeometry {
        self.renderer
            .node_geometry(index, self.node(index), self.graph.node_geometry(index))
    }
    fn node_hit_test(&self, index: usize, point: HitPoint, zoom: f32) -> bool {
        let node = self.node(index);
        let base = self.graph.node_geometry(index);
        let geometry = self.renderer.node_geometry(index, node, base);
        match self
            .renderer
            .node_hit_test(index, node, geometry, point, zoom)
        {
            Some(hit) => hit,
            None if geometry == base => self.graph.node_hit_test(index, point, zoom),
            None => geometry.hit_test_precise(node.position, point, zoom),
        }
    }

    fn node_screen_margin(&self, index: usize) -> f32 {
        self.graph
            .node_screen_margin(index)
            .max(self.renderer.node_screen_margin(index, self.node(index)))
    }
    fn edge_screen_margin(&self, index: usize) -> f32 {
        self.graph
            .edge_screen_margin(index)
            .max(self.renderer.edge_screen_margin(index, self.edge(index)))
    }
    fn edge_pick_radius(&self, index: usize) -> f32 {
        self.renderer.edge_pick_radius(index, self.edge(index))
    }
    fn node_interaction(&self, index: usize) -> NodeInteraction {
        self.graph.node_interaction(index)
    }
    fn edge_selectable(&self, index: usize) -> bool {
        self.graph.edge_selectable(index)
    }
    fn node_ports(&self, index: usize) -> &[Port] {
        self.graph.node_ports(index)
    }
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        self.graph.geometry_changes(since, visit)
    }
    fn resolve(&self, id: ElementId) -> Option<Selection> {
        self.graph.resolve(id)
    }
    fn identity(&self, index: Selection) -> Option<ElementId> {
        self.graph.identity(index)
    }
    fn node_tooltip(&self, ui: &mut Ui, index: usize) {
        self.graph.node_tooltip(ui, index);
    }
    fn edge_tooltip(&self, ui: &mut Ui, index: usize) {
        self.graph.edge_tooltip(ui, index);
    }
}
