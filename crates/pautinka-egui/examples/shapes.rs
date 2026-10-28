mod support;

use egui::{Pos2, Rect, Vec2, pos2};
use pautinka_egui::{
    Canvas, Edge, EdgeId, EdgePath, Graph, GraphId, GraphRevision, Node, NodeId, NodeShape,
    labels::LabelLayoutOptions,
};

struct Gallery {
    id: GraphId,
    nodes: Vec<(&'static str, NodeShape, Pos2)>,
    paths: Vec<EdgePath>,
    bounds: Rect,
}

impl Default for Gallery {
    fn default() -> Self {
        let size = Vec2::new(80.0, 60.0);
        let corner_radius = 12.0.try_into().unwrap();
        let mut nodes = vec![
            ("Circle", NodeShape::circle(35.0).unwrap(), Pos2::ZERO),
            ("Ellipse", NodeShape::ellipse(size).unwrap(), Pos2::ZERO),
        ];
        for (label, rounded_label, shape) in [
            (
                "Rectangle",
                "Rounded rectangle",
                NodeShape::rectangle(size).unwrap(),
            ),
            (
                "Triangle",
                "Rounded triangle",
                NodeShape::triangle(size).unwrap(),
            ),
            (
                "Diamond",
                "Rounded diamond",
                NodeShape::diamond(size).unwrap(),
            ),
            (
                "Pentagon",
                "Rounded pentagon",
                NodeShape::regular_polygon(40.0, 5).unwrap(),
            ),
            (
                "Hexagon",
                "Rounded hexagon",
                NodeShape::regular_polygon(40.0, 6).unwrap(),
            ),
            (
                "Octagon",
                "Rounded octagon",
                NodeShape::regular_polygon(40.0, 8).unwrap(),
            ),
        ] {
            nodes.push((label, shape, Pos2::ZERO));
            nodes.push((
                rounded_label,
                shape.with_corner_radius(corner_radius),
                Pos2::ZERO,
            ));
        }
        let mut bounds = Rect::NOTHING;
        for (index, (_, shape, position)) in nodes.iter_mut().enumerate() {
            *position = pos2((index % 4) as f32 * 200.0, (index / 4) as f32 * 160.0);
            bounds = bounds.union(shape.bounds(*position));
        }
        let paths = nodes
            .windows(2)
            .enumerate()
            .filter(|(index, _)| index % 4 != 3)
            .map(|(_, pair)| {
                let (_, from, a) = pair[0];
                let (_, to, b) = pair[1];
                EdgePath::try_line(
                    from.anchor(a, b - a).unwrap().position(),
                    to.anchor(b, a - b).unwrap().position(),
                )
                .unwrap()
            })
            .collect();
        Self {
            id: GraphId::fresh(),
            nodes,
            paths,
            bounds,
        }
    }
}

impl Graph for Gallery {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }
    fn node_count(&self) -> usize {
        self.nodes.len()
    }
    fn edge_count(&self) -> usize {
        self.paths.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        let (label, _, position) = self.nodes[index];
        Node::new(NodeId::new(index as u64), position, label)
    }
    fn node_shape(&self, index: usize) -> Option<NodeShape> {
        Some(self.nodes[index].1)
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        Edge::new(EdgeId::new(index as u64), &self.paths[index])
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
}

#[derive(Default)]
struct App {
    graph: Gallery,
    canvas: Canvas,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        support::toolbar(ui, &mut self.canvas, &self.graph, "Built-in node shapes");
        ui.weak("Click a shape to select it. Connections end on each shape's contour, including rounded corners.");
        self.canvas.options.interaction = pautinka_egui::InteractionOptions::read_only();
        self.canvas.options.labels = Some(LabelLayoutOptions::default());
        self.canvas.show(ui, &self.graph);
    }
}

fn main() -> eframe::Result {
    support::run("Built-in node shapes", App::default())
}
