use super::*;

use std::cell::Cell;

use pautinka_egui::{
    DefaultRenderer, ElementId, Renderer,
    labels::{
        LabelLayoutOptions, LabelPlacement, LabelPosition, LabelVisibility, NodeLabelOptions,
    },
};

struct LabelsRenderer {
    options: [NodeLabelOptions; 3],
    calls: Cell<usize>,
    style_calls: Cell<usize>,
    paint_calls: Cell<usize>,
    font_size: f32,
}

impl Default for LabelsRenderer {
    fn default() -> Self {
        Self {
            options: [NodeLabelOptions::default(); 3],
            calls: Cell::new(0),
            style_calls: Cell::new(0),
            paint_calls: Cell::new(0),
            font_size: 18.0,
        }
    }
}

impl Renderer for LabelsRenderer {
    fn node_label_options(&self, index: usize, _: Node<'_>) -> NodeLabelOptions {
        self.calls.set(self.calls.get() + 1);
        self.options[index]
    }
    fn node_style(
        &self,
        index: usize,
        node: Node<'_>,
        state: pautinka_egui::ElementState,
        style: &pautinka_egui::Style,
    ) -> pautinka_egui::NodeStyle {
        self.style_calls.set(self.style_calls.get() + 1);
        let mut result = DefaultRenderer.node_style(index, node, state, style);
        result.label_font = egui::FontId::proportional(self.font_size);
        result
    }

    fn paint_node(
        &self,
        context: &pautinka_egui::RenderContext<'_>,
        node: Node<'_>,
        visual: &pautinka_egui::NodeVisual,
    ) {
        self.paint_calls.set(self.paint_calls.get() + 1);
        DefaultRenderer.paint_node(context, node, visual);
    }
}

struct LabelsState {
    graph: TestGraph,
    canvas: Canvas,
    renderer: LabelsRenderer,
}

impl Default for LabelsState {
    fn default() -> Self {
        let mut canvas = Canvas::default();
        canvas.options.labels = Some(LabelLayoutOptions::default());
        canvas.style.label_min_zoom = 0.0;
        Self {
            graph: TestGraph::default(),
            canvas,
            renderer: LabelsRenderer::default(),
        }
    }
}

fn labels_harness(state: LabelsState) -> Harness<'static, LabelsState> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, state: &mut LabelsState| {
                state.canvas.show_with(ui, &state.graph, &state.renderer);
            },
            state,
        )
}

#[test]
fn priority_hidden_override_and_runtime_disable_control_real_painted_labels() {
    let mut state = LabelsState::default();
    state.graph.nodes.fill(pos2(0.0, 0.0));
    for options in &mut state.renderer.options {
        options.placement = LabelPlacement::Fixed(LabelPosition::East);
    }
    state.renderer.options[1].priority = 100;
    let mut harness = labels_harness(state);
    harness.run();
    assert_eq!(harness.state().canvas.node_labels().len(), 1);
    assert_eq!(
        harness.state().canvas.node_labels()[0].id,
        pautinka_egui::NodeId::new(1)
    );
    let texts: Vec<_> = harness
        .output()
        .shapes
        .iter()
        .filter_map(|shape| match &shape.shape {
            egui::Shape::Text(text) => Some(text.galley.text()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["Second"]);
    harness.state_mut().renderer.options[1].visibility = LabelVisibility::Hidden;
    harness.run();
    assert_eq!(
        harness.state().canvas.node_labels()[0].id,
        pautinka_egui::NodeId::new(0)
    );
    harness.state_mut().canvas.options.labels = None;
    let calls = harness.state().renderer.calls.get();
    harness.run();
    assert_eq!(harness.state().renderer.calls.get(), calls);
    assert_eq!(harness.state().canvas.node_labels(), []);
    assert_eq!(harness.state().canvas.label_layout_solves(), 0);
    assert_eq!(
        harness
            .output()
            .shapes
            .iter()
            .filter(|shape| matches!(shape.shape, egui::Shape::Text(_)))
            .count(),
        2
    );
}

#[test]
fn hovering_and_clicking_show_labels_beyond_the_budget_and_can_be_disabled() {
    let mut state = LabelsState::default();
    state.canvas.options.labels.as_mut().unwrap().max_labels = 0;
    let mut harness = labels_harness(state);
    harness.run();
    assert_eq!(harness.state().canvas.node_labels(), []);
    let node = harness.get_by_label("First").rect().center();
    harness.hover_at(node);
    harness.run();
    assert_eq!(harness.state().canvas.node_labels().len(), 1);
    assert!(harness.state().canvas.node_labels()[0].forced);
    click_role_label(&mut harness, egui::accesskit::Role::Button, "First");
    harness.hover_at(pos2(1.0, 1.0));
    harness.run();
    assert_eq!(
        harness.state().canvas.selected_id(),
        Some(ElementId::Node(pautinka_egui::NodeId::new(0)))
    );
    assert_eq!(harness.state().canvas.node_labels().len(), 1);
    harness
        .state_mut()
        .canvas
        .options
        .labels
        .as_mut()
        .unwrap()
        .emphasis
        .selected = false;
    harness.run();
    assert_eq!(harness.state().canvas.node_labels(), []);
    harness.state_mut().renderer.options[1].visibility = LabelVisibility::Always;
    harness.state_mut().canvas.set_labels(false);
    harness.run();
    assert_eq!(
        harness.state().canvas.node_labels()[0].id,
        pautinka_egui::NodeId::new(1)
    );
}

#[test]
fn measured_font_bounds_match_rendering_and_stationary_frames_reuse_placement() {
    let mut state = LabelsState::default();
    state.graph.node_shape =
        Some(pautinka_egui::NodeShape::rectangle(Vec2::new(70.0, 40.0)).unwrap());
    let mut harness = labels_harness(state);
    harness.run();
    let initial = harness.state().canvas.node_labels()[0];
    assert_eq!(
        harness.state().renderer.style_calls.get(),
        harness.state().renderer.paint_calls.get()
    );
    let solves = harness.state().canvas.label_layout_solves();
    for _ in 0..3 {
        harness.step();
    }
    assert_eq!(harness.state().canvas.label_layout_solves(), solves);
    for placement in harness.state().canvas.node_labels() {
        let name = harness.state().graph.node(placement.index).label;
        let text = harness
            .output()
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == name => Some(text),
                _ => None,
            })
            .unwrap();
        assert!(
            placement
                .rect
                .expand(0.01)
                .contains_rect(text.visual_bounding_rect())
        );
        let viewport = harness.get_by_label("Graph canvas").rect();
        for &position in &harness.state().graph.nodes {
            let rect = Rect::from_center_size(
                harness.state().canvas.camera.to_screen(position, viewport),
                Vec2::new(70.0, 40.0) * harness.state().canvas.camera.zoom,
            );
            assert!(!placement.rect.intersects(rect));
        }
    }
    harness.state_mut().renderer.font_size = 30.0;
    harness.run();
    let changed = harness
        .state()
        .canvas
        .node_labels()
        .iter()
        .find(|placed| placed.id == initial.id)
        .unwrap();
    assert!(changed.rect.width() > initial.rect.width());
    assert!(harness.state().canvas.label_layout_solves() > solves);
    harness.state_mut().canvas.reset();
    assert_eq!(harness.state().canvas.node_labels(), []);
}

#[test]
fn invalid_layout_options_return_an_error_before_any_label_callbacks() {
    let mut state = LabelsState::default();
    state.canvas.options.labels.as_mut().unwrap().label_padding = f32::NAN;
    let mut harness = labels_harness(state);
    harness.run();
    assert_eq!(harness.state().renderer.calls.get(), 0);
    assert!(
        harness
            .output()
            .shapes
            .iter()
            .any(|shape| match &shape.shape {
                egui::Shape::Text(text) => text.galley.text().starts_with("Cannot show graph:"),
                _ => false,
            })
    );
}

#[test]
fn slow_pan_and_wheel_zoom_keep_labels_stable_disjoint_and_nodes_clickable() {
    let mut state = LabelsState::default();
    state.graph.nodes = vec![pos2(-20.0, 0.0), pos2(20.0, 0.0)];
    state.canvas.set_camera(pautinka_egui::Camera {
        center: Pos2::ZERO,
        zoom: 1.0,
        fit: false,
    });
    state.canvas.options.camera.limit_zoom_out_to_fit = false;
    let mut harness = labels_harness(state);
    harness.run();
    let initial: Vec<_> = harness
        .state()
        .canvas
        .node_labels()
        .iter()
        .map(|label| (label.id, label.position))
        .collect();
    assert_eq!(initial.len(), 2);
    let viewport = harness.get_by_label("Graph canvas").rect();
    let background = viewport.center() + Vec2::new(0.0, 120.0);
    pointer_gesture(
        &mut harness,
        background,
        background + Vec2::new(3.0, 1.0),
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    harness.hover_at(background);
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, 10.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    for _ in 0..10 {
        harness.step();
        let labels = harness.state().canvas.node_labels();
        assert_eq!(labels.len(), 2);
        for &(id, position) in &initial {
            assert_eq!(
                labels.iter().find(|label| label.id == id).unwrap().position,
                position
            );
        }
        assert!(!labels[0].rect.intersects(labels[1].rect));
    }
    click_role_label(&mut harness, egui::accesskit::Role::Button, "Second");
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(1)));
}
