#![doc = include_str!("../README.ru.md")]

mod crossings;
mod events;
#[cfg(feature = "svg")]
pub mod export;
pub mod geometry;
mod graph;
mod grid;
mod groups;
mod hit;
mod interaction;
pub mod labels;
mod navigator;
mod options;
mod paint;
mod render;
mod response;
mod selection;
mod snapping;
mod spatial;
mod ui;

use egui::{Color32, Pos2, Vec2};
use rustc_hash::FxHashSet;

use hit::HitKey;
use interaction::{Marquee, NodeDrag};
use selection::SelectionState;
use spatial::SpatialCache;

pub use pautinka_core::{
    Alignment, AlignmentIndex, AlignmentSnap, Anchor, AnchorError, ChangeCoverage, CornerRadius,
    EdgeId, ElementId, ElementIndex as Selection, GeometryChanges, GraphId, GraphRevision,
    GroupFrame, GroupFrameError, GroupId, HitPoint, LoopOptions, NodeId, NodeShape, Port, PortId,
    RibbonLane, RibbonSpacing, RibbonSpacingError, RouteEndpoints, RoutingMode, ShapeError,
    ShapeKind, SnapGrid, SnapGuide, SnapIndex, SnapKind, SnapKinds, SnapOptions, SnapResult,
    SnapTarget, SpatialError,
};

pub use crossings::{
    BridgeSide, CrossingAngle, CrossingMode, CrossingOptions, CrossingOrder, CrossingPoints,
    CrossingSize, EdgeCrossings,
};
pub use spatial::{CanvasStats, PreparedGraph};

pub use events::CanvasEvent;
pub use graph::{Direction, Edge, Graph, Node};
pub use grid::{GridKind, GridOptions, GridSnapMode, paint_grid};
pub use groups::{
    Group, GroupInteraction, GroupState, GroupStyle, GroupVisual, paint_default_group,
};
pub use navigator::{NavigatorCorner, NavigatorOptions};
pub use options::{
    CameraOptions, CanvasOptions, DetailLevel, InteractionOptions, LodOptions, ModifierKey,
    NodeInteraction, ZoomRange,
};
pub use render::{
    DefaultRenderer, EdgeStyle, ElementState, LinePattern, NodeGeometry, NodeStyle, NodeVisual,
    RenderContext, Renderer, paint_default_node,
};
pub use response::{CanvasError, CanvasResponse};

pub use geometry::{Camera, Curve, EdgePath, PathError, PathSample};

/// Оформление холста и общие параметры подписей.
#[derive(Clone)]
pub struct Style {
    pub background: Color32,
    pub selection: Color32,
    pub highlight: Color32,
    pub label_color: Color32,
    pub label_min_zoom: f32,
    pub crossings: CrossingOptions,
    pub accessibility_label: String,
    pub empty_text: String,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: Color32::from_rgb(20, 25, 34),
            selection: Color32::WHITE,
            highlight: Color32::GOLD,
            label_color: Color32::from_rgb(218, 227, 237),
            label_min_zoom: 1.0,
            crossings: CrossingOptions::default(),
            accessibility_label: "Graph canvas".into(),
            empty_text: "No nodes".into(),
        }
    }
}

/// Состояние интерактивного холста: камера, подписи, видимость петель и выбор.
pub struct Canvas {
    navigator_cache: navigator::NavigatorCache,
    label_cache: Option<Box<labels::LabelCache>>,
    snapping: snapping::SnapCache,
    pub camera: Camera,
    labels: bool,
    show_loops: bool,
    selection: SelectionState,
    spatial: SpatialCache,
    crossings: crossings::CrossingCache,
    crossing_overrides: std::collections::BTreeMap<EdgeId, EdgeCrossings>,
    marquee: Option<Marquee>,
    focus_requested: bool,
    hovered: Option<ElementId>,
    context_target: Option<ElementId>,
    context_position: Option<Pos2>,
    hit_cache: Option<(HitKey, Option<Selection>)>,
    revision: Option<GraphRevision>,
    highlighted: FxHashSet<ElementId>,
    viewport_size: Vec2,
    initial_fit_complete: bool,
    events: Vec<CanvasEvent>,
    notified_selection: u64,
    renderer_key: Option<render::RendererKey>,
    detail: DetailLevel,
    pub options: CanvasOptions,
    drag: Option<NodeDrag>,
    group_drag: Option<groups::GroupDrag>,
    pending_focus: Option<Selection>,
    pub style: Style,
}

impl Canvas {
    /// Восстанавливает подготовленную камеру без начального вписывания документа.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
        self.initial_fit_complete = true;
        self.pending_focus = None;
    }

    /// События последнего вызова `show`; буфер переиспользуется между кадрами.
    pub fn events(&self) -> &[CanvasEvent] {
        &self.events
    }

    /// Подсветка связанных объектов не меняет локальный выбор и положение камеры.
    pub fn set_highlighted(&mut self, elements: impl IntoIterator<Item = ElementId>) {
        self.highlighted.clear();
        self.highlighted.extend(elements);
    }

    pub fn is_highlighted(&self, element: ElementId) -> bool {
        self.highlighted.contains(&element)
    }

    pub fn labels(&self) -> bool {
        self.labels
    }

    pub fn set_labels(&mut self, labels: bool) {
        self.labels = labels;
    }

    pub fn show_loops(&self) -> bool {
        self.show_loops
    }

    /// Сбрасывает выбор и откладывает вписывание нового графа до следующего кадра.
    /// Вызывать при замене графа другим документом; перестановка элементов сохраняет выбор по ID.
    pub fn reset(&mut self) {
        self.navigator_cache = navigator::NavigatorCache::default();
        self.label_cache = None;
        self.snapping = snapping::SnapCache::default();
        self.selection.clear();
        self.spatial = SpatialCache::default();
        self.crossings = crossings::CrossingCache::default();
        self.crossing_overrides.clear();
        self.revision = None;
        self.highlighted.clear();
        self.initial_fit_complete = false;
        self.drag = None;
        self.group_drag = None;
        self.marquee = None;
        self.hovered = None;
        self.context_target = None;
        self.context_position = None;
        self.renderer_key = None;
        self.hit_cache = None;
        self.events.clear();
        self.pending_focus = None;
        self.camera.fit = true;
    }

    fn visible_edges<'a>(
        &'a self,
        graph: &'a (impl Graph + ?Sized),
    ) -> impl Iterator<Item = (usize, Edge<'a>)> {
        (0..graph.edge_count())
            .map(|index| (index, graph.edge(index)))
            .filter(|(_, edge)| self.show_loops || !edge.is_loop)
    }
}
impl Default for Canvas {
    fn default() -> Self {
        Self {
            navigator_cache: navigator::NavigatorCache::default(),
            label_cache: None,
            snapping: snapping::SnapCache::default(),
            camera: Camera::default(),
            labels: true,
            show_loops: true,
            selection: SelectionState::default(),
            spatial: SpatialCache::default(),
            crossings: crossings::CrossingCache::default(),
            crossing_overrides: std::collections::BTreeMap::new(),
            marquee: None,
            focus_requested: false,
            hovered: None,
            context_target: None,
            context_position: None,
            hit_cache: None,
            revision: None,
            highlighted: FxHashSet::default(),
            viewport_size: Vec2::ZERO,
            initial_fit_complete: false,
            events: Vec::with_capacity(4),
            notified_selection: 0,
            renderer_key: None,
            detail: DetailLevel::Full,
            options: CanvasOptions::default(),
            drag: None,
            group_drag: None,
            pending_focus: None,
            style: Style::default(),
        }
    }
}
