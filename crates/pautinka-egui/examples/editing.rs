mod support;

use egui::{Pos2, Vec2};
use pautinka_egui::{Canvas, CanvasEvent, ElementId, NodeId, SnapKinds};

struct Drag {
    origin: Pos2,
    before: Vec<(NodeId, Pos2)>,
}
#[derive(Default)]
struct App {
    graph: support::Network,
    canvas: Canvas,
    drag: Option<Drag>,
    undo: Vec<Vec<(NodeId, Pos2)>>,
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        support::toolbar(ui, &mut self.canvas, &self.graph, "Graph editing");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo gesture"))
                .clicked()
                && let Some(before) = self.undo.pop()
            {
                for (id, position) in before {
                    self.graph.move_node(id, position);
                }
            }
            ui.checkbox(&mut self.canvas.options.interaction.snapping, "Snapping");
            let mut diagonals = self
                .canvas
                .options
                .snapping
                .kinds
                .contains(SnapKinds::DIAGONALS);
            if ui
                .add_enabled(
                    self.canvas.options.interaction.snapping,
                    egui::Checkbox::new(&mut diagonals, "45° guides"),
                )
                .changed()
            {
                self.canvas
                    .options
                    .snapping
                    .kinds
                    .set(SnapKinds::DIAGONALS, diagonals);
            }
        });
        ui.weak("Drag a node to move it, or select a group first. Tank is pinned. Escape cancels the gesture.");
        self.canvas.show(ui, &self.graph);
        for &event in self.canvas.events() {
            match event {
                CanvasEvent::NodeDragStarted { position, .. } => {
                    // Снимок выделенной группы создаётся один раз на жест, а не на каждом кадре.
                    let before = self
                        .canvas
                        .selected_elements()
                        .filter_map(|element| match element {
                            ElementId::Node(id) => self.graph.position(id).map(|p| (id, p)),
                            ElementId::Edge(_) | ElementId::Group(_) => None,
                        })
                        .collect();
                    self.drag = Some(Drag {
                        origin: position,
                        before,
                    });
                }
                CanvasEvent::NodeDragged { position, .. }
                | CanvasEvent::NodeDragFinished { position, .. } => {
                    if let Some(drag) = &self.drag {
                        let delta = position - drag.origin;
                        for &(id, before) in &drag.before {
                            if id != NodeId::new(30) {
                                self.graph.move_node(id, before + delta);
                            }
                        }
                    }
                    if matches!(event, CanvasEvent::NodeDragFinished { .. })
                        && let Some(drag) = self.drag.take()
                        && position - drag.origin != Vec2::ZERO
                    {
                        self.undo.push(drag.before);
                    }
                }
                CanvasEvent::NodeDragCancelled { .. } => {
                    if let Some(drag) = self.drag.take() {
                        for (id, position) in drag.before {
                            self.graph.move_node(id, position);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
fn main() -> eframe::Result {
    support::run("Event-driven graph editing", App::default())
}
