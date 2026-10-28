use std::{
    fmt,
    num::NonZeroUsize,
    str::FromStr,
    time::{Duration, Instant},
};

use egui::{Color32, Pos2, Rect, Vec2};
use pautinka_egui::{
    Canvas, CanvasEvent, ChangeCoverage, Direction, Edge, EdgeId, GeometryChanges, Graph, GraphId,
    GraphRevision, Node, NodeId, RouteEndpoints, RoutingMode, Selection, geometry::EdgePath,
};

use crate::cpu::CpuUiHarness;
pub use crate::cpu::FrameSample;

/// Детерминированные сцены для функциональных тестов и CPU-профилирования.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Network,
    Dense,
    Parallel,
    Loops,
    Hub,
    Labels,
    Orthogonal,
}

impl SceneKind {
    pub const ALL: [Self; 7] = [
        Self::Network,
        Self::Dense,
        Self::Parallel,
        Self::Loops,
        Self::Hub,
        Self::Labels,
        Self::Orthogonal,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::Dense => "dense",
            Self::Parallel => "parallel",
            Self::Loops => "loops",
            Self::Hub => "hub",
            Self::Labels => "labels",
            Self::Orthogonal => "orthogonal",
        }
    }
}

impl FromStr for SceneKind {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_str() == value)
            .ok_or("Unknown scene")
    }
}

impl fmt::Display for SceneKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub struct Scene {
    id: GraphId,
    kind: SceneKind,
    revision: GraphRevision,
    changes: GeometryChanges,
    incident_offsets: Vec<usize>,
    incident_edges: Vec<usize>,
    pub positions: Vec<Pos2>,
    paths: Vec<EdgePath>,
    ends: Vec<[usize; 2]>,
    labels: Vec<String>,
    bounds: Rect,
}

impl Scene {
    pub fn new(kind: SceneKind, count: NonZeroUsize) -> Self {
        let count = count.get();
        let columns = count.isqrt().max(1);
        let spacing = if kind == SceneKind::Dense { 12.0 } else { 80.0 };
        let positions: Vec<_> = (0..count)
            .map(|i| {
                egui::pos2(
                    (i % columns) as f32 * spacing,
                    (i / columns) as f32 * spacing,
                )
            })
            .collect();
        let labels = (0..count)
            .map(|i| {
                if kind == SceneKind::Labels {
                    format!("Узел {i}: газораспределительная станция — Северный промышленный район")
                } else {
                    format!("Node {i}")
                }
            })
            .collect();
        let mut paths = Vec::with_capacity(count * 3);
        let mut ends = Vec::with_capacity(count * 3);
        for a in 0..count {
            for lane in 0..3 {
                let b = match kind {
                    SceneKind::Parallel => (a + 1) % count,
                    SceneKind::Loops if lane == 0 => a,
                    SceneKind::Hub if lane == 0 => 0,
                    SceneKind::Dense => (a + 1 + lane * 7) % count,
                    _ => (a + [1, columns, columns + 1][lane]) % count,
                };
                let path = route(kind, &positions, a, b, lane);
                paths.push(path);
                ends.push([a, b]);
            }
        }
        let bounds = paths
            .iter()
            .fold(Rect::NOTHING, |bounds, path| bounds.union(path.bounds()))
            .expand(60.0);
        let mut incident_offsets = vec![0; count + 1];
        for &[a, b] in &ends {
            incident_offsets[a + 1] += 1;
            if a != b {
                incident_offsets[b + 1] += 1;
            }
        }
        for i in 1..incident_offsets.len() {
            incident_offsets[i] += incident_offsets[i - 1];
        }
        let mut next = incident_offsets[..count].to_vec();
        let mut incident_edges = vec![0; *incident_offsets.last().unwrap()];
        for (edge, &[a, b]) in ends.iter().enumerate() {
            incident_edges[next[a]] = edge;
            next[a] += 1;
            if a != b {
                incident_edges[next[b]] = edge;
                next[b] += 1;
            }
        }
        Self {
            id: GraphId::fresh(),
            kind,
            revision: GraphRevision::default(),
            changes: GeometryChanges::default(),
            incident_offsets,
            incident_edges,
            positions,
            paths,
            ends,
            labels,
            bounds,
        }
    }

    /// Пересчитываются только инцидентные пути; bounds расширяются консервативно, без полного сканирования.
    pub fn move_node(&mut self, index: usize, position: Pos2) {
        if self.positions[index] == position {
            return;
        }
        self.positions[index] = position;
        let incident =
            &self.incident_edges[self.incident_offsets[index]..self.incident_offsets[index + 1]];
        for &edge in incident {
            let [a, b] = self.ends[edge];
            self.paths[edge] = route(self.kind, &self.positions, a, b, edge % 3);
            self.bounds = self.bounds.union(self.paths[edge].bounds());
        }
        let before = self.revision;
        self.revision = before.next_geometry();
        self.changes.record(
            before,
            self.revision,
            std::iter::once(Selection::Node(index))
                .chain(incident.iter().copied().map(Selection::Edge)),
        );
    }

    pub fn visible_counts(&self, canvas: &Canvas, viewport: Rect) -> (usize, usize) {
        let bounds = Rect::from_min_max(
            canvas.camera.to_world(viewport.min, viewport),
            canvas.camera.to_world(viewport.max, viewport),
        );
        (
            self.positions
                .iter()
                .filter(|&&position| bounds.contains(position))
                .count(),
            self.paths
                .iter()
                .filter(|path| bounds.intersects(path.bounds()))
                .count(),
        )
    }
}

impl Graph for Scene {
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        self.changes.visit(since, self.revision, visit)
    }

    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }

    fn node_count(&self) -> usize {
        self.positions.len()
    }
    fn edge_count(&self) -> usize {
        self.paths.len()
    }
    fn node(&self, index: usize) -> Node<'_> {
        Node {
            id: NodeId::new(index as u64),
            position: self.positions[index],
            label: &self.labels[index],
            accessibility_label: &self.labels[index],
            radius: 5.0,
            color: Color32::LIGHT_BLUE,
        }
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        Edge {
            id: EdgeId::new(index as u64),
            path: &self.paths[index],
            color: Color32::LIGHT_GREEN,
            direction: Direction::Forward,
            is_loop: self.ends[index][0] == self.ends[index][1],
        }
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
}

/// CPU-harness без GPU и дерева AccessKit; тесселяция измеряется отдельно.
pub struct CpuCanvasHarness {
    pub scene: Scene,
    pub canvas: Canvas,
    pub viewport: Rect,
    ui: CpuUiHarness,
}

impl CpuCanvasHarness {
    pub fn prepare(&mut self) -> Duration {
        let start = Instant::now();
        self.canvas
            .prepare(&self.scene)
            .expect("Generated scene has valid geometry");
        start.elapsed()
    }
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            canvas: Canvas::default(),
            viewport: Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 1000.0)),
            ui: CpuUiHarness::default(),
        }
    }

    pub fn frame(&mut self, events: Vec<egui::Event>) -> FrameSample {
        self.ui.frame(self.viewport, events, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                self.canvas.show(ui, &self.scene);
            });
            for event in self.canvas.events() {
                if let CanvasEvent::NodeDragged { node, position }
                | CanvasEvent::NodeDragFinished { node, position } = *event
                {
                    self.scene.move_node(node.get() as usize, position);
                }
            }
        })
    }
}

fn route(kind: SceneKind, positions: &[Pos2], a: usize, b: usize, lane: usize) -> EdgePath {
    let offset = (lane as f32 - 1.0) * 24.0;
    if kind == SceneKind::Orthogonal {
        let endpoints = if a == b {
            RouteEndpoints::Loop {
                position: positions[a],
            }
        } else {
            RouteEndpoints::Between {
                from: positions[a],
                to: positions[b],
            }
        };
        RoutingMode::Orthogonal
            .route(endpoints, offset, lane)
            .expect("Generated orthogonal route has valid geometry")
    } else {
        EdgePath::new(positions[a], positions[b], offset, lane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_scenes_are_deterministic_finite_and_have_three_edges_per_node() {
        for kind in SceneKind::ALL {
            for count in [1, 2, 1000] {
                let scene = Scene::new(kind, NonZeroUsize::new(count).unwrap());
                let other = Scene::new(kind, NonZeroUsize::new(count).unwrap());
                assert_eq!(scene.positions, other.positions);
                assert_eq!(scene.ends, other.ends);
                assert_eq!(scene.edge_count(), count * 3);
                assert!(scene.bounds.is_finite());
                for (path, other) in scene.paths.iter().zip(&other.paths) {
                    assert_eq!(path.points(), other.points());
                    assert!(path.bounds().is_finite());
                    if kind == SceneKind::Orthogonal {
                        assert!(matches!(
                            path.curve(),
                            pautinka_egui::geometry::Curve::Polyline(_)
                        ));
                        for pair in path.points().windows(2) {
                            assert!(pair[0].x == pair[1].x || pair[0].y == pair[1].y);
                        }
                    }
                }
            }
        }
    }
}
