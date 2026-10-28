use egui::{Color32, Rect, Vec2, pos2};

use crate::{
    DefaultRenderer, Direction, Edge, EdgeId, EdgePath, GraphId, GraphRevision, Node, NodeId,
    NodeShape, NodeVisual, RenderContext, RouteEndpoints, RoutingMode,
};

use super::*;

struct Diagram {
    id: GraphId,
    paths: Vec<EdgePath>,
    labels: [&'static str; 2],
}

impl Default for Diagram {
    fn default() -> Self {
        Self {
            id: GraphId::fresh(),
            paths: vec![
                EdgePath::try_line(pos2(-50.0, 0.0), pos2(50.0, 0.0)).unwrap(),
                EdgePath::try_line(pos2(0.0, -50.0), pos2(0.0, 50.0)).unwrap(),
                EdgePath::try_quadratic([pos2(-50.0, 0.0), pos2(0.0, 90.0), pos2(50.0, 0.0)])
                    .unwrap(),
                RoutingMode::Curved
                    .route(
                        RouteEndpoints::Loop {
                            position: pos2(50.0, 0.0),
                        },
                        0.0,
                        0,
                    )
                    .unwrap(),
            ],
            labels: ["Насос <A> & B", "Клапан\nValve"],
        }
    }
}

impl Graph for Diagram {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }
    fn node_count(&self) -> usize {
        self.labels.len()
    }
    fn edge_count(&self) -> usize {
        self.paths.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node::new(
            NodeId::new(index as u64),
            pos2(if index == 0 { -50.0 } else { 50.0 }, 0.0),
            self.labels[index],
        )
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        let mut edge = Edge::new(EdgeId::new(index as u64), &self.paths[index]);
        edge.direction = Direction::Both;
        edge.is_loop = index == 3;
        edge
    }
    fn bounds(&self) -> Rect {
        self.paths.iter().fold(
            Rect::from_min_max(pos2(-50.0, -50.0), pos2(50.0, 50.0)),
            |bounds, path| bounds.union(path.bounds()),
        )
    }
    fn node_shape(&self, index: usize) -> Option<NodeShape> {
        (index == 1).then(|| NodeShape::rectangle(Vec2::new(30.0, 20.0)).unwrap())
    }
}

#[test]
fn group_only_export_includes_frames_and_uses_the_group_renderer() {
    use crate::{Group, GroupFrame, GroupId, GroupState, GroupVisual};
    use std::cell::Cell;

    struct Groups {
        id: GraphId,
        frame: GroupFrame,
    }
    impl Graph for Groups {
        fn id(&self) -> GraphId {
            self.id
        }
        fn revision(&self) -> GraphRevision {
            GraphRevision::default()
        }
        fn node_count(&self) -> usize {
            0
        }
        fn edge_count(&self) -> usize {
            0
        }
        fn node(&self, _: usize) -> Node<'_> {
            panic!("No nodes")
        }
        fn edge(&self, _: usize) -> Edge<'_> {
            panic!("No edges")
        }
        fn bounds(&self) -> Rect {
            Rect::ZERO
        }
        fn group_count(&self) -> usize {
            1
        }
        fn group(&self, _: usize) -> Group<'_> {
            let mut group = Group::new(GroupId::new(0), self.frame, "Control cabinet");
            group.state = Some(GroupState::Collapsed);
            group
        }
    }
    struct Recorder(Cell<usize>);
    impl Renderer for Recorder {
        fn paint_group(&self, context: &RenderContext<'_>, group: Group<'_>, visual: &GroupVisual) {
            self.0.set(self.0.get() + 1);
            crate::paint_default_group(context, group, visual);
        }
    }
    let graph = Groups {
        id: GraphId::fresh(),
        frame: GroupFrame::new(
            Rect::from_min_size(pos2(200.0, 100.0), Vec2::new(180.0, 80.0)),
            24.0,
        )
        .unwrap(),
    };
    let renderer = Recorder(Cell::new(0));
    let exporter = Exporter::default();
    let options = ExportOptions::default();
    let svg = exporter
        .svg(&graph, &renderer, &Style::default(), &options)
        .unwrap();
    assert!(renderer.0.get() > 0);
    assert!(svg.contains("<path"));
    #[cfg(feature = "png")]
    assert!(
        exporter
            .png(&graph, &renderer, &Style::default(), &options, 1.0)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
}
#[test]
fn export_uses_the_requested_label_layout_and_validates_its_options() {
    use crate::labels::{LabelLayoutOptions, NodeLabelVisual};
    use std::cell::Cell;

    struct Recorder(Cell<usize>);
    impl Renderer for Recorder {
        fn paint_node_label(
            &self,
            context: &RenderContext<'_>,
            _: Node<'_>,
            visual: &NodeLabelVisual<'_>,
        ) {
            self.0.set(self.0.get() + 1);
            crate::labels::paint_node_label(context, visual);
        }
    }
    let graph = Diagram::default();
    let renderer = Recorder(Cell::new(0));
    let exporter = Exporter::default();
    let options = ExportOptions::default()
        .with_label_layout(Some(LabelLayoutOptions::default()))
        .unwrap();
    let svg = exporter
        .svg(&graph, &renderer, &Style::default(), &options)
        .unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(renderer.0.get() > 0);
    renderer.0.set(0);
    exporter
        .svg(
            &graph,
            &renderer,
            &Style::default(),
            &options.with_labels(false),
        )
        .unwrap();
    assert_eq!(renderer.0.get(), 0);
    assert!(matches!(
        ExportOptions::default().with_label_layout(Some(LabelLayoutOptions {
            switch_margin: f32::NAN,
            ..LabelLayoutOptions::default()
        })),
        Err(ExportError::InvalidOptions)
    ));
}

#[test]
fn labels_are_self_contained_vector_outlines_and_do_not_require_xml_escaping() {
    let exporter = Exporter::default();
    let graph = Diagram::default();
    let options = ExportOptions::default();
    let with_labels = exporter
        .svg(&graph, &DefaultRenderer, &Style::default(), &options)
        .unwrap();
    let without = exporter
        .svg(
            &graph,
            &DefaultRenderer,
            &Style::default(),
            &options.with_labels(false),
        )
        .unwrap();
    assert!(with_labels.starts_with("<svg xmlns="));
    assert!(!with_labels.contains("<text"));
    assert!(!with_labels.contains("<image"));
    assert!(!with_labels.contains("Насос"));
    assert!(with_labels.matches("<path").count() > without.matches("<path").count() + 10);
    assert!(with_labels.contains(" Q"));
    assert!(with_labels.contains(" C"));
    assert!(with_labels.contains("<circle"));
    assert_eq!(
        with_labels,
        exporter
            .svg(
                &graph,
                &DefaultRenderer,
                &Style::default(),
                &ExportOptions::default()
            )
            .unwrap()
    );
}

#[test]
fn styled_text_preserves_utf8_section_boundaries_across_empty_sections_and_rows() {
    struct Styled;
    impl Renderer for Styled {
        fn paint_node(&self, context: &RenderContext<'_>, _: Node<'_>, visual: &NodeVisual) {
            let mut job = egui::text::LayoutJob::default();
            for (text, color) in [
                ("Ж\n", Color32::from_rgb(1, 2, 3)),
                ("", Color32::from_rgb(4, 5, 6)),
                ("\n", Color32::from_rgb(7, 8, 9)),
                ("Б\t", Color32::from_rgb(10, 11, 12)),
                ("C", Color32::from_rgb(13, 14, 15)),
            ] {
                job.append(
                    text,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(11.0),
                        color,
                        ..egui::TextFormat::default()
                    },
                );
            }
            let galley = context.painter.layout_job(job);
            context
                .painter
                .galley(visual.rect.min, galley, Color32::WHITE);
        }
    }
    let svg = Exporter::default()
        .svg(
            &Diagram::default(),
            &Styled,
            &Style::default(),
            &ExportOptions::default(),
        )
        .unwrap();
    for color in ["#010203", "#0a0b0c", "#0d0e0f"] {
        assert_eq!(svg.matches(&format!("fill=\"{color}\"")).count(), 2);
    }
    for color in ["#040506", "#070809"] {
        assert!(!svg.contains(color));
    }
}

#[test]
fn invalid_options_fonts_and_unrepresentable_transforms_return_errors() {
    for value in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        assert!(ExportOptions::new(value, 0.0).is_err());
    }
    assert!(ExportOptions::new(1.0, -1.0).is_err());
    assert!(ExportOptions::default().with_region(Rect::ZERO).is_err());
    assert!(Exporter::new(FontDefinitions::empty()).is_err());
    assert!(
        Exporter::default()
            .svg(
                &Diagram::default(),
                &DefaultRenderer,
                &Style::default(),
                &ExportOptions::new(f32::MAX, 1.0).unwrap()
            )
            .is_err()
    );
}

#[test]
fn custom_renderer_and_crossing_decorations_are_exported() {
    struct Custom;
    impl Renderer for Custom {
        fn paint_node(&self, context: &RenderContext<'_>, _: Node<'_>, visual: &NodeVisual) {
            context
                .painter
                .circle_filled(visual.rect.center(), 13.0, Color32::from_rgb(1, 2, 3));
        }
    }
    let exporter = Exporter::default();
    let graph = Diagram::default();
    let mut style = Style::default();
    style.crossings.mode = crate::CrossingMode::None;
    let plain = exporter
        .svg(&graph, &Custom, &style, &ExportOptions::default())
        .unwrap();
    style.crossings.mode = crate::CrossingMode::Gap;
    let gaps = exporter
        .svg(&graph, &Custom, &style, &ExportOptions::default())
        .unwrap();
    style.crossings.mode = crate::CrossingMode::Bridge;
    let bridges = exporter
        .svg(&graph, &Custom, &style, &ExportOptions::default())
        .unwrap();
    assert!(plain.contains("#010203"));
    assert!(gaps.matches("<path").count() > plain.matches("<path").count());
    assert!(bridges.matches("<path").count() > gaps.matches("<path").count());
}

#[test]
fn bezier_crossing_exports_retain_curve_commands_for_gaps_and_bridges() {
    let graph = Diagram {
        paths: vec![
            EdgePath::try_quadratic([pos2(-100.0, 40.0), pos2(0.0, -40.0), pos2(100.0, 40.0)])
                .unwrap(),
            EdgePath::try_cubic([
                pos2(-40.0, -100.0),
                pos2(40.0 / 3.0, -100.0 / 3.0),
                pos2(40.0 / 3.0, 100.0 / 3.0),
                pos2(-40.0, 100.0),
            ])
            .unwrap(),
        ],
        ..Diagram::default()
    };
    let exporter = Exporter::default();
    let mut style = Style::default();
    for (mode, quadratics, cubics) in [
        (crate::CrossingMode::None, 1, 1),
        (crate::CrossingMode::Gap, 1, 2),
        (crate::CrossingMode::Bridge, 2, 1),
    ] {
        style.crossings.mode = mode;
        let svg = exporter
            .svg(&graph, &DefaultRenderer, &style, &ExportOptions::default())
            .unwrap();
        assert_eq!(svg.matches(" Q").count(), quadratics);
        assert_eq!(svg.matches(" C").count(), cubics);
    }
}

#[test]
fn dotted_edge_patterns_export_circles_and_dashes_for_paths_and_bridges() {
    use crate::{EdgeStyle, ElementState, LinePattern};

    struct PatternRenderer(LinePattern);
    impl Renderer for PatternRenderer {
        fn edge_style(
            &self,
            index: usize,
            edge: Edge<'_>,
            state: ElementState,
            style: &Style,
        ) -> EdgeStyle {
            EdgeStyle {
                pattern: self.0,
                ..DefaultRenderer.edge_style(index, edge, state, style)
            }
        }
        fn paint_node(&self, _: &RenderContext<'_>, _: Node<'_>, _: &NodeVisual) {}
    }
    let graph = Diagram::default();
    let exporter = Exporter::default();
    let options = ExportOptions::default().with_labels(false);
    let mut style = Style::default();
    style.crossings.mode = crate::CrossingMode::Bridge;
    for pattern in [
        LinePattern::Dotted { gap: 4.0 },
        LinePattern::DashDot {
            dash: 8.0,
            gap: 4.0,
        },
        LinePattern::DashDotDot {
            dash: 8.0,
            gap: 4.0,
        },
    ] {
        let svg = exporter
            .svg(&graph, &PatternRenderer(pattern), &style, &options)
            .unwrap();
        assert!(svg.matches("<circle").count() > 4);
        assert!(svg.contains("<path"));
    }
}

#[test]
fn unsupported_custom_painting_fails_instead_of_disappearing() {
    struct Textured;
    impl Renderer for Textured {
        fn paint_node(&self, context: &RenderContext<'_>, _: Node<'_>, _: &NodeVisual) {
            let mut mesh = egui::epaint::Mesh::default();
            mesh.add_colored_rect(context.viewport, Color32::RED);
            context.painter.add(egui::Shape::mesh(mesh));
        }
    }
    assert!(matches!(
        Exporter::default().svg(
            &Diagram::default(),
            &Textured,
            &Style::default(),
            &ExportOptions::default()
        ),
        Err(ExportError::UnsupportedShape(_))
    ));
}

#[cfg(feature = "png")]
#[test]
fn png_region_resolution_transparency_and_svg_parity() {
    let exporter = Exporter::default();
    let graph = Diagram::default();
    let options = ExportOptions::new(1.0, 99.0)
        .unwrap()
        .with_region(Rect::from_min_max(pos2(-100.0, -80.0), pos2(100.0, 80.0)))
        .unwrap()
        .with_background(None);
    let style = Style::default();
    let svg = exporter
        .svg(&graph, &DefaultRenderer, &style, &options)
        .unwrap();
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
    assert_eq!((tree.size().width(), tree.size().height()), (200.0, 160.0));
    let png = exporter
        .png(&graph, &DefaultRenderer, &style, &options, 2.0)
        .unwrap();
    let image = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert_eq!((image.width(), image.height()), (400, 320));
    assert_eq!(image.pixel(0, 0).unwrap().alpha(), 0);
    assert!(image.pixels().iter().any(|pixel| pixel.alpha() > 0));
    let mut expected = resvg::tiny_skia::Pixmap::new(400, 320).unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(2.0, 2.0),
        &mut expected.as_mut(),
    );
    assert_eq!(image, expected);
    let opaque = exporter
        .png(
            &graph,
            &DefaultRenderer,
            &style,
            &options.with_background(Some(Color32::from_rgb(17, 34, 51))),
            1.0,
        )
        .unwrap();
    assert_eq!(
        resvg::tiny_skia::Pixmap::decode_png(&opaque)
            .unwrap()
            .pixel(0, 0)
            .unwrap()
            .alpha(),
        255
    );
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY, 10_000.0] {
        assert!(
            exporter
                .png(
                    &graph,
                    &DefaultRenderer,
                    &style,
                    &ExportOptions::default(),
                    scale
                )
                .is_err()
        );
    }
}

#[cfg(feature = "png")]
#[test]
fn full_export_fits_labels_and_padding_and_loop_toggle_changes_content() {
    let exporter = Exporter::default();
    let graph = Diagram::default();
    let style = Style {
        background: Color32::from_rgb(21, 43, 65),
        ..Style::default()
    };
    let render = |options: &ExportOptions| {
        let svg = exporter
            .svg(&graph, &DefaultRenderer, &style, options)
            .unwrap();
        let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default()).unwrap();
        (svg, tree.size())
    };
    let (all, size) = render(&ExportOptions::new(1.0, 0.0).unwrap());
    let (_, padded) = render(&ExportOptions::new(1.0, 8.0).unwrap());
    assert!((padded.width() - size.width() - 16.0).abs() < 0.001);
    assert!((padded.height() - size.height() - 16.0).abs() < 0.001);
    assert!(all.contains("#152b41"));
    let (no_loops, _) = render(&ExportOptions::default().with_loops(false));
    assert!(!no_loops.contains(" C"));
    let (_, no_labels) = render(&ExportOptions::new(1.0, 0.0).unwrap().with_labels(false));
    assert!(size.width() > no_labels.width());
}

#[test]
fn empty_diagram_exports_without_ui_placeholder() {
    struct Empty;
    impl Graph for Empty {
        fn id(&self) -> GraphId {
            GraphId::fresh()
        }
        fn revision(&self) -> GraphRevision {
            GraphRevision::default()
        }
        fn node_count(&self) -> usize {
            0
        }
        fn edge_count(&self) -> usize {
            0
        }
        fn node(&self, _: usize) -> Node<'_> {
            unreachable!()
        }
        fn edge(&self, _: usize) -> Edge<'_> {
            unreachable!()
        }
        fn bounds(&self) -> Rect {
            Rect::ZERO
        }
    }
    let svg = Exporter::default()
        .svg(
            &Empty,
            &DefaultRenderer,
            &Style::default(),
            &ExportOptions::default().with_background(None),
        )
        .unwrap();
    assert!(!svg.contains("<path"));
    assert!(!svg.contains("No nodes"));
    assert!(!svg.contains("<rect"));
}

#[test]
fn a_node_at_large_absolute_coordinates_has_finite_export_bounds() {
    struct SingleNode {
        id: GraphId,
        position: egui::Pos2,
    }
    impl Graph for SingleNode {
        fn id(&self) -> GraphId {
            self.id
        }
        fn revision(&self) -> GraphRevision {
            GraphRevision::default()
        }
        fn node_count(&self) -> usize {
            1
        }
        fn edge_count(&self) -> usize {
            0
        }
        fn node(&self, _: usize) -> Node<'_> {
            Node::new(NodeId::new(0), self.position, "Node")
        }
        fn edge(&self, _: usize) -> Edge<'_> {
            unreachable!()
        }
        fn bounds(&self) -> Rect {
            Rect::from_min_max(self.position, self.position)
        }
    }
    let exporter = Exporter::default();
    for coordinate in [2.0e38, -2.0e38, f32::from_bits(1)] {
        let graph = SingleNode {
            id: GraphId::fresh(),
            position: pos2(coordinate, coordinate),
        };
        let svg = exporter
            .svg(
                &graph,
                &DefaultRenderer,
                &Style::default(),
                &ExportOptions::default(),
            )
            .unwrap();
        assert!(svg.contains("<circle"));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }
}

#[test]
fn built_in_shapes_export_their_contours_with_the_default_renderer() {
    struct ShapedNode {
        id: GraphId,
        shape: NodeShape,
    }
    impl Graph for ShapedNode {
        fn id(&self) -> GraphId {
            self.id
        }
        fn revision(&self) -> GraphRevision {
            GraphRevision::default()
        }
        fn node_count(&self) -> usize {
            1
        }
        fn edge_count(&self) -> usize {
            0
        }
        fn node(&self, _: usize) -> Node<'_> {
            Node::new(NodeId::new(0), egui::Pos2::ZERO, "Shape")
        }
        fn node_shape(&self, _: usize) -> Option<NodeShape> {
            Some(self.shape)
        }
        fn edge(&self, _: usize) -> Edge<'_> {
            panic!("No edges")
        }
        fn bounds(&self) -> Rect {
            self.shape.bounds(egui::Pos2::ZERO)
        }
    }
    let size = Vec2::new(100.0, 80.0);
    let mut shapes = vec![
        NodeShape::circle(40.0).unwrap(),
        NodeShape::ellipse(size).unwrap(),
    ];
    for shape in [
        NodeShape::rectangle(size).unwrap(),
        NodeShape::triangle(size).unwrap(),
        NodeShape::diamond(size).unwrap(),
    ]
    .into_iter()
    .chain((3..=8).map(|sides| NodeShape::regular_polygon(50.0, sides).unwrap()))
    {
        shapes.push(shape);
        shapes.push(shape.with_corner_radius(20.0.try_into().unwrap()));
    }
    let options = ExportOptions::default()
        .with_labels(false)
        .with_background(None)
        .with_region(Rect::from_center_size(egui::Pos2::ZERO, Vec2::splat(120.0)))
        .unwrap();
    let exporter = Exporter::default();
    for shape in shapes {
        let graph = ShapedNode {
            id: GraphId::fresh(),
            shape,
        };
        let svg = exporter
            .svg(&graph, &DefaultRenderer, &Style::default(), &options)
            .unwrap();
        let element = match shape.kind() {
            crate::ShapeKind::Circle => "<circle",
            crate::ShapeKind::Ellipse => "<ellipse",
            _ => "<path",
        };
        assert!(svg.contains(element), "{shape:?}");
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{shape:?}");
        #[cfg(feature = "png")]
        {
            let png = exporter
                .png(&graph, &DefaultRenderer, &Style::default(), &options, 1.0)
                .unwrap();
            let pixmap = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
            assert_eq!(pixmap.pixel(60, 60).unwrap().alpha(), 255, "{shape:?}");
            assert_eq!(pixmap.pixel(1, 1).unwrap().alpha(), 0, "{shape:?}");
            if shape.kind() != crate::ShapeKind::Rectangle || shape.corner_radius().get() > 0.0 {
                assert_eq!(pixmap.pixel(109, 21).unwrap().alpha(), 0, "{shape:?}");
            }
        }
    }
}
