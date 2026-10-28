use egui::{Color32, Pos2, Rect, Vec2};
use pautinka::egui::{
    Canvas, DefaultRenderer, Edge, EdgeId, ElementId, Graph, GraphId, GraphRevision, Node, NodeId,
    NodeVisual, PreparedGraph, RenderContext, Renderer,
};
use pautinka::{EdgePath, NodeShape, pos2};

struct Model {
    id: GraphId,
    path: EdgePath,
    shape: NodeShape,
}
impl Graph for Model {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }
    fn node_count(&self) -> usize {
        2
    }
    fn edge_count(&self) -> usize {
        1
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(
            NodeId::new([20, 60][index]),
            [pos2(-100.0, 0.0), pos2(100.0, 0.0)][index],
            ["Source", "Consumer"][index],
        )
    }
    fn node_shape(&self, _: usize) -> Option<NodeShape> {
        Some(self.shape)
    }
    fn edge(&self, _: usize) -> Edge<'_> {
        Edge::new(EdgeId::new(100), &self.path)
    }
    fn bounds(&self) -> Rect {
        Rect::from_min_max(pos2(-120.0, -10.0), pos2(120.0, 10.0))
    }
}
struct Symbols;
impl Renderer for Symbols {
    fn paint_node(&self, context: &RenderContext<'_>, node: Node<'_>, visual: &NodeVisual) {
        DefaultRenderer.paint_node(context, node, visual);
        context
            .painter
            .circle_filled(visual.rect.center(), 2.0, Color32::GREEN);
    }
}
fn prepare_model(shape: NodeShape, path: EdgePath) -> (Model, PreparedGraph) {
    let model = Model {
        id: GraphId::fresh(),
        path,
        shape,
    };
    let renderer: &dyn Renderer = &Symbols;
    let prepared = PreparedGraph::new_with(&model, renderer).unwrap();
    (model, prepared)
}
pub fn run(shape: NodeShape, path: EdgePath) {
    #[cfg(not(target_arch = "wasm32"))]
    let (model, prepared) = std::thread::spawn(move || prepare_model(shape, path))
        .join()
        .unwrap();
    #[cfg(target_arch = "wasm32")]
    let (model, prepared) = prepare_model(shape, path);
    let mut canvas = Canvas::default();
    let renderer: &dyn Renderer = &Symbols;
    assert!(canvas.install_prepared_with(prepared, &model, renderer));
    let rebuilds = canvas.stats().rebuilds;
    assert!(canvas.select_id(Some(ElementId::Node(NodeId::new(20))), &model));
    let context = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
        ..Default::default()
    };
    let mut output = context.run_ui(input, |ui| {
        let graph: &dyn Graph = &model;
        let renderer: &dyn Renderer = &Symbols;
        let response = canvas.try_show_with(ui, graph, renderer).unwrap();
        let point = pos2(10.0, 20.0);
        assert!(response.to_world(response.to_screen(point)).distance(point) < 0.001);
    });
    output.textures_delta.clear();
    let meshes = context.tessellate(output.shapes, output.pixels_per_point);
    assert!(!meshes.is_empty());
    assert_eq!(canvas.stats().rebuilds, rebuilds);
    assert_eq!(canvas.selected_id(), Some(ElementId::Node(NodeId::new(20))));
    println!("Independent Pautinka consumer passed");
}
