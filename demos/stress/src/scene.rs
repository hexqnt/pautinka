mod config;
mod generate;
mod graph;

use std::{f64::consts::TAU, time::Duration};

use egui::{Color32, Pos2, Rect, Vec2};
use pautinka::egui::{
    CanvasEvent, Direction, GeometryChanges, GraphId, GraphRevision, NodeId, NodeShape,
    RouteEndpoints, RoutingMode, Selection, geometry::EdgePath,
};

pub(crate) use config::NodeCount;
use config::PALETTE;
pub use config::{Appearance, Load, Motion, NodeKind, STAGES, SceneStyle, Topology};

struct Particle {
    center: Pos2,
    position: Pos2,
    phase: f64,
    frequency: f64,
    amplitude: Vec2,
    moving_rank: u8,
    label: String,
}

impl Particle {
    fn orbit(&self) -> Vec2 {
        let (sin, cos) = self.phase.sin_cos();
        Vec2::new(cos as f32 * self.amplitude.x, sin as f32 * self.amplitude.y)
    }
}

struct Link {
    ends: [usize; 2],
    lane: u8,
    path: EdgePath,
}
struct Drag {
    index: usize,
    center: Pos2,
    position: Pos2,
}

pub struct Scene {
    id: GraphId,
    revision: GraphRevision,
    changes: GeometryChanges,
    seed: u32,
    load: Load,
    topology: Topology,
    nodes: Vec<Particle>,
    edges: Vec<Link>,
    dirty: Vec<bool>,
    changed: Vec<Selection>,
    bounds: Rect,
    drag: Option<Drag>,
    style: SceneStyle,
    shapes: [NodeShape; 5],
    edge_colors: [Color32; 12],
}

impl Scene {
    pub fn new(seed: u32, load: Load) -> Self {
        Self::with_style(seed, load, SceneStyle::default())
    }

    pub fn with_style(seed: u32, load: Load, style: SceneStyle) -> Self {
        Self::with_options(seed, load, style, Topology::default())
    }

    pub fn with_options(seed: u32, load: Load, style: SceneStyle, topology: Topology) -> Self {
        let mut scene = Self {
            id: GraphId::fresh(),
            revision: GraphRevision::default().next_topology(),
            changes: GeometryChanges::with_capacity((load.nodes() + load.edge_count()) * 2),
            seed,
            load,
            topology,
            nodes: Vec::with_capacity(load.nodes()),
            edges: Vec::with_capacity(load.edge_count()),
            dirty: vec![false; load.nodes()],
            changed: Vec::with_capacity(load.nodes() + load.edge_count()),
            bounds: Rect::NOTHING,
            drag: None,
            style,
            shapes: style.appearance.shapes(),
            edge_colors: PALETTE.map(|color| color.gamma_multiply(0.48)),
        };
        scene.generate(load);
        scene.update_bounds();
        scene
    }

    pub fn load(&self) -> Load {
        self.load
    }
    pub fn topology(&self) -> Topology {
        self.topology
    }
    pub fn seed(&self) -> u32 {
        self.seed
    }
    pub fn style(&self) -> SceneStyle {
        self.style
    }
    pub fn appearance(&self) -> Appearance {
        self.style.appearance
    }
    pub fn routing(&self) -> RoutingMode {
        self.style.routing
    }
    pub fn direction(&self) -> Direction {
        self.style.direction
    }
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub fn resize(&mut self, load: Load) {
        self.reconfigure(load, self.topology);
    }

    pub fn reconfigure(&mut self, load: Load, topology: Topology) {
        if self.load == load && self.topology == topology {
            return;
        }
        self.cancel_drag();
        self.topology = topology;
        self.generate(load);
        self.load = load;
        self.dirty.resize(load.nodes(), false);
        self.changed.clear();
        self.changed.reserve(load.nodes() + load.edge_count());
        // Два полных пакета покрывают анимацию перед показом и перенос после показа без выпадения истории.
        self.changes = GeometryChanges::with_capacity((load.nodes() + load.edge_count()) * 2);
        self.revision = self.revision.next_topology();
        self.update_bounds();
    }

    /// Все изменения оформления публикуются одной версией геометрии.
    pub fn set_style(&mut self, style: SceneStyle) {
        if self.style == style {
            return;
        }
        self.changed.clear();
        if self.style.appearance.radius() != style.appearance.radius() {
            self.shapes = style.appearance.shapes();
        }
        if self.style.appearance.kind != style.appearance.kind
            || self.style.appearance.radius() != style.appearance.radius()
        {
            self.changed
                .extend((0..self.nodes.len()).map(Selection::Node));
        }
        if self.style.routing != style.routing || self.style.corner_radius != style.corner_radius {
            for edge in &mut self.edges {
                edge.path = route(style, &self.nodes, edge.ends, edge.lane);
            }
            self.changed
                .extend((0..self.edges.len()).map(Selection::Edge));
            self.update_bounds();
        }
        self.style = style;
        if self.changed.is_empty() {
            self.revision = self.revision.next_appearance();
        } else {
            self.publish_changes();
        }
    }

    pub fn set_appearance(&mut self, appearance: Appearance) {
        self.set_style(SceneStyle {
            appearance,
            ..self.style
        });
    }
    pub fn set_routing(&mut self, routing: RoutingMode) {
        self.set_style(SceneStyle {
            routing,
            ..self.style
        });
    }
    pub fn set_direction(&mut self, direction: Direction) {
        self.set_style(SceneStyle {
            direction,
            ..self.style
        });
    }

    fn shape(&self, index: usize) -> NodeShape {
        self.shapes[self.style.appearance.kind.shape_index(index)]
    }
    fn publish_changes(&mut self) {
        let before = self.revision;
        self.revision = before.next_geometry();
        self.changes
            .record(before, self.revision, self.changed.iter().copied());
    }

    /// Шаг времени задаётся вызывающим кодом; замороженные и захваченные вершины сохраняют фазу.
    pub fn advance(&mut self, step: Duration, motion: Motion) {
        if step.is_zero() || motion.speed == 0.0 || motion.fraction == 0 {
            return;
        }
        let seconds = step.as_secs_f64() * f64::from(motion.speed);
        for (index, (node, dirty)) in self.nodes.iter_mut().zip(&mut self.dirty).enumerate() {
            if node.moving_rank >= motion.fraction
                || self.drag.as_ref().is_some_and(|drag| drag.index == index)
            {
                continue;
            }
            node.phase += seconds * node.frequency;
            if node.phase >= TAU {
                node.phase %= TAU;
            }
            let position = node.center + node.orbit();
            *dirty = position != node.position;
            node.position = position;
        }
        self.commit_positions();
    }

    pub fn handle_event(&mut self, event: CanvasEvent) {
        match event {
            CanvasEvent::NodeDragStarted { node, .. } => {
                let Some(index) = self.node_index(node) else {
                    return;
                };
                self.cancel_drag();
                let particle = &self.nodes[index];
                self.drag = Some(Drag {
                    index,
                    center: particle.center,
                    position: particle.position,
                });
            }
            CanvasEvent::NodeDragged { node, position }
            | CanvasEvent::NodeDragFinished { node, position } => {
                if !position.is_finite() || position.x.abs().max(position.y.abs()) > 1_000_000.0 {
                    return;
                }
                let Some(drag) = &self.drag else {
                    return;
                };
                if node.get() != drag.index as u64 {
                    return;
                }
                let index = drag.index;
                let particle = &mut self.nodes[index];
                self.dirty[index] = particle.position != position;
                particle.position = position;
                particle.center = position - particle.orbit();
                self.commit_positions();
                if matches!(event, CanvasEvent::NodeDragFinished { .. }) {
                    self.drag = None;
                }
            }
            CanvasEvent::NodeDragCancelled { .. } => self.cancel_drag(),
            _ => {}
        }
    }
    fn node_index(&self, id: NodeId) -> Option<usize> {
        usize::try_from(id.get())
            .ok()
            .filter(|&index| index < self.nodes.len())
    }
    fn cancel_drag(&mut self) {
        if let Some(drag) = self.drag.take() {
            let node = &mut self.nodes[drag.index];
            self.dirty[drag.index] = node.position != drag.position;
            node.center = drag.center;
            node.position = drag.position;
            self.commit_positions();
        }
    }
    fn commit_positions(&mut self) {
        self.changed.clear();
        if !self.dirty.iter().any(|&dirty| dirty) {
            return;
        }
        let mut bounds = Rect::NOTHING;
        for (index, (node, &dirty)) in self.nodes.iter().zip(&self.dirty).enumerate() {
            if dirty {
                self.changed.push(Selection::Node(index));
            }
            bounds = bounds.union(Rect::from_center_size(node.position, Vec2::splat(40.0)));
        }
        // Границы собираются в тех же проходах, что и изменения: отдельный обход крупных EdgePath не нужен.
        for (index, edge) in self.edges.iter_mut().enumerate() {
            let [a, b] = edge.ends;
            if self.dirty[a] || self.dirty[b] {
                edge.path = route(self.style, &self.nodes, edge.ends, edge.lane);
                self.changed.push(Selection::Edge(index));
            }
            bounds = bounds.union(edge.path.bounds());
        }
        self.dirty.fill(false);
        self.publish_changes();
        self.bounds = bounds;
    }
    fn update_bounds(&mut self) {
        let bounds = self.nodes.iter().fold(Rect::NOTHING, |bounds, node| {
            bounds.union(Rect::from_center_size(node.position, Vec2::splat(40.0)))
        });
        self.bounds = self
            .edges
            .iter()
            .fold(bounds, |bounds, edge| bounds.union(edge.path.bounds()));
    }
    pub fn visible_counts(&self, bounds: Rect) -> (usize, usize) {
        (
            self.nodes
                .iter()
                .enumerate()
                .filter(|&(index, node)| bounds.intersects(self.shape(index).bounds(node.position)))
                .count(),
            self.edges
                .iter()
                .filter(|edge| bounds.intersects(edge.path.bounds()))
                .count(),
        )
    }
}

fn route(style: SceneStyle, nodes: &[Particle], [a, b]: [usize; 2], lane: u8) -> EdgePath {
    style
        .routing
        .route(
            RouteEndpoints::Between {
                from: nodes[a].position,
                to: nodes[b].position,
            },
            if lane.is_multiple_of(2) { 12.0 } else { -12.0 },
            0,
        )
        .expect("Generated geometry must be finite")
        .with_corner_radius(style.corner_radius)
        .expect("Rounded geometry must be finite")
}

#[cfg(test)]
mod tests;
