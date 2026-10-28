mod geometry;
mod graph;
mod node;

use egui::{Align2, Color32, FontId, Painter, Rect, Stroke, Vec2};

use crate::{Edge, Node, NodeShape, Style, geometry::Camera, options::DetailLevel};

pub use geometry::NodeGeometry;
pub(crate) use graph::{RenderedGraph, RendererKey};

/// Один renderer используется для рисования, пространственных границ и выбора. При изменении геометрии renderer увеличивайте `revision`.
pub trait Renderer {
    fn group_screen_margin(&self, _index: usize, _group: crate::Group<'_>) -> f32 {
        8.0
    }
    fn group_style(
        &self,
        _index: usize,
        group: crate::Group<'_>,
        state: ElementState,
        style: &Style,
    ) -> crate::GroupStyle {
        crate::GroupStyle {
            fill: group.color.gamma_multiply(0.07),
            header_fill: group.color.gamma_multiply(0.16),
            stroke: Stroke::new(if state.hovered { 2.0 } else { 1.0 }, group.color),
            halo: state.halo(style, 3.0),
            label_color: style.label_color,
            label_font: FontId::proportional(12.0),
        }
    }
    /// Геометрия рамки и заголовка остаётся общей для рисования и взаимодействия.
    fn paint_group(
        &self,
        context: &RenderContext<'_>,
        group: crate::Group<'_>,
        visual: &crate::GroupVisual,
    ) {
        crate::paint_default_group(context, group, visual);
    }
    /// Подложка интерактивного холста после заливки, перед рёбрами и узлами; painter обрезан областью просмотра.
    /// Собственный метод может вызвать `DefaultRenderer.paint_background(context, grid)` для сохранения сетки.
    fn paint_background(&self, context: &RenderContext<'_>, grid: Option<&crate::GridOptions>) {
        if let Some(grid) = grid {
            crate::paint_grid(context, grid);
        }
    }
    /// Настройки читаются только при включённой автоматической раскладке. Шрифт и цвет задаёт `node_style`.
    fn node_label_options(
        &self,
        _index: usize,
        _node: Node<'_>,
    ) -> crate::labels::NodeLabelOptions {
        crate::labels::NodeLabelOptions::default()
    }
    /// Умные подписи рисуются после узлов; `paint_node` получает `show_label = false`.
    fn paint_node_label(
        &self,
        context: &RenderContext<'_>,
        _node: Node<'_>,
        visual: &crate::labels::NodeLabelVisual<'_>,
    ) {
        crate::labels::paint_node_label(context, visual);
    }
    /// Версия пространственного представления; при замене объекта за `dyn Renderer` владелец также меняет версию.
    fn revision(&self) -> u64 {
        0
    }
    fn node_geometry(
        &self,
        _index: usize,
        _node: Node<'_>,
        geometry: NodeGeometry,
    ) -> NodeGeometry {
        geometry
    }
    /// `Some` полностью задаёт результат выбора; `None` сохраняет проверку модели при прежней геометрии либо прямоугольник `NodeGeometry` при изменённой.
    /// Точка задана в мировых координатах; `relative_to(node.position)` сохраняет малые смещения при больших абсолютных позициях.
    fn node_hit_test(
        &self,
        _index: usize,
        _node: Node<'_>,
        _geometry: NodeGeometry,
        _point: crate::HitPoint,
        _zoom: f32,
    ) -> Option<bool> {
        None
    }
    /// Запас в экранных точках для подписей, halo и дополнительных символов за пределами геометрии узла.
    fn node_screen_margin(&self, _index: usize, node: Node<'_>) -> f32 {
        32.0 + node.label.len() as f32 * 11.0
    }
    fn edge_screen_margin(&self, _index: usize, _edge: Edge<'_>) -> f32 {
        16.0
    }
    /// Допуск выбора в экранных точках, независимо от масштаба камеры.
    fn edge_pick_radius(&self, _index: usize, _edge: Edge<'_>) -> f32 {
        7.0
    }
    fn node_style(
        &self,
        _index: usize,
        node: Node<'_>,
        state: ElementState,
        style: &Style,
    ) -> NodeStyle {
        NodeStyle {
            fill: node.color,
            stroke: Stroke::new(1.0, style.background),
            halo: state.halo(style, 2.0),
            label_color: style.label_color,
            label_font: FontId::proportional(11.0),
            label_offset: Vec2::new(10.0, -8.0),
        }
    }
    fn edge_style(
        &self,
        _index: usize,
        edge: Edge<'_>,
        state: ElementState,
        style: &Style,
    ) -> EdgeStyle {
        EdgeStyle {
            stroke: Stroke::new(if state.emphasized() { 2.2 } else { 1.6 }, edge.color),
            halo: state.halo(style, 5.0),
            arrow_size: Vec2::new(7.0, 3.5),
            pattern: LinePattern::Solid,
        }
    }
    fn edge_label(&self, _index: usize) -> Option<&str> {
        None
    }
    fn paint_node(&self, context: &RenderContext<'_>, node: Node<'_>, visual: &NodeVisual) {
        paint_default_node(context, node, visual);
    }
    /// Дополнительные маркеры и условные обозначения рисуются поверх штатного пути ребра.
    fn paint_edge_overlay(
        &self,
        _context: &RenderContext<'_>,
        _index: usize,
        _edge: Edge<'_>,
        _state: ElementState,
    ) {
    }
}

/// Рисунок линии в экранных точках. Некорректные длины дают сплошную линию.
/// Диаметр точек равен толщине основного штриха; halo сохраняет их центры и интервалы.
#[derive(Clone, Copy, Debug, Default)]
pub enum LinePattern {
    #[default]
    Solid,
    /// Длина штриха и промежутка в экранных точках; некорректные значения дают сплошную линию.
    Dashed { dash: f32, gap: f32 },
    /// Круглые точки; `gap` задаёт расстояние между краями соседних точек.
    Dotted { gap: f32 },
    /// Штрих и одна точка; `gap` задаёт промежуток между соседними элементами.
    DashDot { dash: f32, gap: f32 },
    /// Штрих и две точки; `gap` задаёт промежуток между соседними элементами.
    DashDotDot { dash: f32, gap: f32 },
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ElementState {
    pub selected: bool,
    pub highlighted: bool,
    pub hovered: bool,
}

impl ElementState {
    pub fn emphasized(self) -> bool {
        self.selected || self.highlighted || self.hovered
    }

    fn halo(self, style: &Style, width: f32) -> Option<Stroke> {
        let color = if self.selected {
            style.selection
        } else if self.highlighted {
            style.highlight
        } else {
            return None;
        };
        Some(Stroke::new(width, color))
    }
}

#[derive(Clone, Debug)]
pub struct NodeStyle {
    pub fill: Color32,
    pub stroke: Stroke,
    pub halo: Option<Stroke>,
    pub label_color: Color32,
    pub label_font: FontId,
    pub label_offset: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeStyle {
    pub stroke: Stroke,
    pub halo: Option<Stroke>,
    /// Длина и половина ширины наконечника в экранных точках.
    pub arrow_size: Vec2,
    pub pattern: LinePattern,
}

impl EdgeStyle {
    pub(crate) fn strokes(self) -> impl Iterator<Item = Stroke> {
        self.halo.into_iter().chain(std::iter::once(self.stroke))
    }

    pub(crate) fn outer_width(self) -> f32 {
        self.halo
            .map_or(self.stroke.width, |halo| halo.width.max(self.stroke.width))
    }
}

pub struct RenderContext<'a> {
    pub painter: &'a Painter,
    pub camera: Camera,
    pub viewport: Rect,
    pub style: &'a Style,
    pub detail: DetailLevel,
}

pub struct NodeVisual {
    pub index: usize,
    pub rect: Rect,
    pub shape: Option<NodeShape>,
    pub state: ElementState,
    pub style: NodeStyle,
    pub show_label: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultRenderer;
impl Renderer for DefaultRenderer {}

/// Можно переиспользовать штатное рисование в собственном renderer и дорисовать предметные символы.
#[inline]
pub fn paint_default_node(context: &RenderContext<'_>, node: Node<'_>, visual: &NodeVisual) {
    let painter = context.painter;
    let style = &visual.style;
    let stroke = if context.detail == DetailLevel::Overview && !visual.state.emphasized() {
        Stroke::NONE
    } else {
        style.stroke
    };
    node::paint(context, visual, stroke);
    if visual.show_label {
        painter.text(
            context.camera.to_screen(node.position, context.viewport) + style.label_offset,
            Align2::LEFT_BOTTOM,
            node.label,
            style.label_font.clone(),
            style.label_color,
        );
    }
}
