mod support;

use egui::{Pos2, Rect, Vec2, pos2};
use pautinka_egui::{
    Canvas, CanvasEvent, Edge, EdgeId, EdgePath, Graph, GraphId, GraphRevision, Group, GroupFrame,
    GroupId, GroupState, Node, NodeId, NodeShape,
};

/// Предметная модель и её видимое представление принадлежат приложению.
struct Network {
    id: GraphId,
    revision: GraphRevision,
    positions: [Pos2; 3],
    collapsed: [bool; 2],
    frames: [GroupFrame; 2],
    paths: Vec<EdgePath>,
    visible_nodes: Vec<usize>,
}

fn shape() -> NodeShape {
    NodeShape::rectangle(Vec2::new(70.0, 36.0)).unwrap()
}

impl Default for Network {
    fn default() -> Self {
        let frame =
            GroupFrame::new(Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0)), 24.0).unwrap();
        let mut model = Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default(),
            positions: [pos2(-160.0, 0.0), pos2(-40.0, 0.0), pos2(160.0, 0.0)],
            collapsed: [false; 2],
            frames: [frame; 2],
            paths: Vec::with_capacity(2),
            visible_nodes: Vec::with_capacity(3),
        };
        model.rebuild();
        model
    }
}

impl Network {
    fn rebuild(&mut self) {
        self.paths.clear();
        self.visible_nodes.clear();
        let cabinet = GroupFrame::enclosing(
            self.positions[..2].iter().map(|&p| shape().bounds(p)),
            Vec2::splat(20.0),
            24.0,
        )
        .unwrap();
        self.frames[1] = if self.collapsed[1] {
            GroupFrame::new(
                Rect::from_min_size(cabinet.bounds().min, Vec2::new(160.0, 48.0)),
                24.0,
            )
            .unwrap()
        } else {
            cabinet
        };
        let plant = GroupFrame::enclosing(
            [self.frames[1].bounds(), shape().bounds(self.positions[2])],
            Vec2::splat(30.0),
            24.0,
        )
        .unwrap();
        self.frames[0] = if self.collapsed[0] {
            GroupFrame::new(
                Rect::from_min_size(plant.bounds().min, Vec2::new(200.0, 48.0)),
                24.0,
            )
            .unwrap()
        } else {
            plant
        };
        if self.collapsed[0] {
            return;
        }
        if self.collapsed[1] {
            self.visible_nodes.push(2);
            // Владелец явно заменяет скрытый конец внешней связи границей свёрнутого блока.
            let from = self.frames[1].bounds().right_center();
            let to = shape()
                .anchor(self.positions[2], -Vec2::X)
                .unwrap()
                .position();
            self.paths.push(EdgePath::try_line(from, to).unwrap());
        } else {
            self.visible_nodes.extend(0..3);
            for pair in self.positions.windows(2) {
                let from = shape().anchor(pair[0], Vec2::X).unwrap().position();
                let to = shape().anchor(pair[1], -Vec2::X).unwrap().position();
                self.paths.push(EdgePath::try_line(from, to).unwrap());
            }
        }
    }

    fn set_positions(&mut self, positions: [Pos2; 3]) {
        if self.positions != positions {
            self.positions = positions;
            self.rebuild();
            self.revision = self.revision.next_geometry();
        }
    }

    fn move_group(&mut self, group: GroupId, before: [Pos2; 3], delta: Vec2) {
        // Правила принадлежности известны только этой модели; цех содержит все три узла.
        let count = if group == GroupId::new(0) { 3 } else { 2 };
        let mut positions = self.positions;
        for (position, origin) in positions[..count].iter_mut().zip(before) {
            *position = origin + delta;
        }
        self.set_positions(positions);
    }
}

impl Graph for Network {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn node_count(&self) -> usize {
        self.visible_nodes.len()
    }
    fn edge_count(&self) -> usize {
        self.paths.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        let index = self.visible_nodes[index];
        Node::new(
            NodeId::new(index as u64),
            self.positions[index],
            ["Controller", "Relay", "Drive"][index],
        )
    }
    fn node_shape(&self, _: usize) -> Option<NodeShape> {
        Some(shape())
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        Edge::new(
            EdgeId::new(if self.collapsed[1] { 1 } else { index as u64 }),
            &self.paths[index],
        )
    }
    fn group_count(&self) -> usize {
        if self.collapsed[0] { 1 } else { 2 }
    }
    fn group(&self, index: usize) -> Group<'_> {
        let mut group = Group::new(
            GroupId::new(index as u64),
            self.frames[index],
            ["Workshop", "Control cabinet"][index],
        );
        group.state = Some(if self.collapsed[index] {
            GroupState::Collapsed
        } else {
            GroupState::Expanded
        });
        group
    }
    fn bounds(&self) -> Rect {
        self.frames[0].bounds()
    }
}

struct Drag {
    group: GroupId,
    before: [Pos2; 3],
}

#[derive(Default)]
struct App {
    graph: Network,
    canvas: Canvas,
    drag: Option<Drag>,
    undo: Vec<[Pos2; 3]>,
    ignore_requests: bool,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        support::toolbar(ui, &mut self.canvas, &self.graph, "Nested groups");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.ignore_requests, "Ignore group requests");
            if ui
                .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo move"))
                .clicked()
                && let Some(positions) = self.undo.pop()
            {
                self.graph.set_positions(positions);
            }
        });
        ui.weak("Drag a group header. Escape cancels. The application owns movement and collapse; ignore requests to keep the model unchanged.");
        self.canvas.show(ui, &self.graph);
        for &event in self.canvas.events() {
            match event {
                CanvasEvent::GroupCollapseRequested { group, collapsed }
                    if !self.ignore_requests =>
                {
                    self.graph.collapsed[group.get() as usize] = collapsed;
                    self.graph.rebuild();
                    self.graph.revision = self.graph.revision.next_topology();
                }
                CanvasEvent::GroupDragStarted { group } if !self.ignore_requests => {
                    self.drag = Some(Drag {
                        group,
                        before: self.graph.positions,
                    });
                }
                CanvasEvent::GroupDragged { delta, .. }
                | CanvasEvent::GroupDragFinished { delta, .. } => {
                    if let Some(drag) = &self.drag {
                        self.graph.move_group(drag.group, drag.before, delta);
                    }
                    if matches!(event, CanvasEvent::GroupDragFinished { .. })
                        && let Some(drag) = self.drag.take()
                        && self.graph.positions != drag.before
                    {
                        self.undo.push(drag.before);
                    }
                }
                CanvasEvent::GroupDragCancelled { .. } => {
                    if let Some(drag) = self.drag.take() {
                        self.graph.move_group(drag.group, drag.before, Vec2::ZERO);
                    }
                }
                _ => {}
            }
        }
    }
}

fn main() -> eframe::Result {
    let mut app = App::default();
    app.canvas.options.interaction.node_drag = false;
    support::run("Application-owned groups", app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapse_preserves_domain_positions_and_external_edge_identity() {
        let mut graph = Network::default();
        let positions = graph.positions;
        let edge = graph.edge(1).id;
        graph.collapsed[1] = true;
        graph.rebuild();
        assert_eq!(graph.positions, positions);
        assert_eq!(graph.visible_nodes, [2]);
        assert_eq!(graph.edge(0).id, edge);
        assert_eq!(
            graph.paths[0].points()[0],
            graph.frames[1].bounds().right_center()
        );
        graph.collapsed[0] = true;
        graph.rebuild();
        assert_eq!(graph.group_count(), 1);
        assert_eq!((graph.node_count(), graph.edge_count()), (0, 0));
        graph.collapsed = [false; 2];
        graph.rebuild();
        assert_eq!(graph.positions, positions);
        assert_eq!(
            (graph.node_count(), graph.edge_count(), graph.group_count()),
            (3, 2, 2)
        );
    }

    #[test]
    fn model_applies_total_delta_once_and_can_restore_the_snapshot() {
        let mut graph = Network::default();
        let before = graph.positions;
        graph.move_group(GroupId::new(1), before, Vec2::new(10.0, 20.0));
        graph.move_group(GroupId::new(1), before, Vec2::new(30.0, 40.0));
        let revision = graph.revision;
        graph.move_group(GroupId::new(1), before, Vec2::new(30.0, 40.0));
        assert_eq!(graph.revision, revision);
        assert_eq!(
            graph.positions,
            [
                before[0] + Vec2::new(30.0, 40.0),
                before[1] + Vec2::new(30.0, 40.0),
                before[2]
            ]
        );
        graph.move_group(GroupId::new(1), before, Vec2::ZERO);
        assert_eq!(graph.positions, before);
        graph.move_group(GroupId::new(0), before, Vec2::splat(50.0));
        assert_eq!(
            graph.positions,
            before.map(|position| position + Vec2::splat(50.0))
        );
    }
}
