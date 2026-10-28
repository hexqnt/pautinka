use egui::{Color32, Pos2, Rect, Ui};

use crate::{
    ChangeCoverage, EdgeId, EdgePath, ElementId, GraphId, GraphRevision, Group, GroupInteraction,
    NodeGeometry, NodeId, NodeInteraction, NodeShape, Port, Selection,
};

/// Доступ к данным графа без промежуточных коллекций и копирования.
///
/// Индексы должны быть плотными: `0..node_count()` и `0..edge_count()`.
/// При изменении порядка элементов увеличивайте версию топологии; ID сохраняют идентичность.
/// При замене графа другим документом вызовите [`crate::Canvas::reset`].
/// Геометрия и границы подготавливаются владельцем данных вне отрисовки.
pub trait Graph {
    fn id(&self) -> GraphId;
    fn revision(&self) -> GraphRevision;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
    fn node(&self, index: usize) -> Node<'_>;
    fn edge(&self, index: usize) -> Edge<'_>;
    fn bounds(&self) -> Rect;
    /// Видимые рамки в порядке рисования: внешние перед внутренними. Принадлежность и скрытие содержимого задаёт приложение.
    fn group_count(&self) -> usize {
        0
    }
    fn group(&self, _index: usize) -> Group<'_> {
        panic!("Group index out of bounds")
    }
    fn group_interaction(&self, _index: usize) -> GroupInteraction {
        GroupInteraction::default()
    }
    fn group_screen_margin(&self, _index: usize) -> f32 {
        8.0
    }
    fn group_tooltip(&self, ui: &mut Ui, index: usize) {
        ui.label(self.group(index).label);
    }
    /// Владелец перечисляет все изменения после `since` либо запрашивает полную перестройку кешей.
    fn geometry_changes(
        &self,
        _since: GraphRevision,
        _visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        ChangeCoverage::Unavailable
    }

    /// `None` сохраняет круг с экранным радиусом из `Node`; явная форма задаётся в мировых координатах.
    fn node_shape(&self, _index: usize) -> Option<NodeShape> {
        None
    }
    fn node_geometry(&self, index: usize) -> NodeGeometry {
        NodeGeometry::for_node(self.node(index), self.node_shape(index))
    }
    fn node_hit_test(&self, index: usize, point: crate::HitPoint, zoom: f32) -> bool {
        let node = self.node(index);
        let geometry = self.node_geometry(index);
        if let Some(shape) = self.node_shape(index)
            && geometry == NodeGeometry::for_node(node, Some(shape))
        {
            return geometry.hit_test_shape(node.position, point, zoom, shape);
        }
        geometry.hit_test_precise(node.position, point, zoom)
    }
    fn node_screen_margin(&self, index: usize) -> f32 {
        32.0 + self.node_geometry(index).screen_margin()
    }
    fn edge_screen_margin(&self, _index: usize) -> f32 {
        16.0
    }
    fn edge_pick_radius(&self, _index: usize) -> f32 {
        7.0
    }
    fn node_interaction(&self, _index: usize) -> NodeInteraction {
        NodeInteraction::default()
    }
    fn edge_selectable(&self, _index: usize) -> bool {
        true
    }
    fn node_ports(&self, _index: usize) -> &[Port] {
        &[]
    }
    fn resolve(&self, id: ElementId) -> Option<Selection> {
        match id {
            ElementId::Node(id) => (0..self.node_count())
                .find(|&index| self.node(index).id == id)
                .map(Selection::Node),
            ElementId::Edge(id) => (0..self.edge_count())
                .find(|&index| self.edge(index).id == id)
                .map(Selection::Edge),
            ElementId::Group(id) => (0..self.group_count())
                .find(|&index| self.group(index).id == id)
                .map(Selection::Group),
        }
    }

    fn identity(&self, selection: Selection) -> Option<ElementId> {
        match selection {
            Selection::Node(index) if index < self.node_count() => {
                Some(ElementId::Node(self.node(index).id))
            }
            Selection::Edge(index) if index < self.edge_count() => {
                Some(ElementId::Edge(self.edge(index).id))
            }
            Selection::Group(index) if index < self.group_count() => {
                Some(ElementId::Group(self.group(index).id))
            }
            _ => None,
        }
    }

    /// Содержимое всплывающей подсказки узла.
    fn node_tooltip(&self, ui: &mut Ui, index: usize) {
        ui.label(self.node(index).label);
    }
    fn edge_tooltip(&self, ui: &mut Ui, index: usize) {
        ui.label(format!("Edge {}", self.edge(index).id.get()));
    }
}
/// Направления стрелок на пути ребра.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Undirected,
    Forward,
    Reverse,
    Both,
}

impl Direction {
    pub(crate) const fn arrows(self) -> &'static [bool] {
        match self {
            Self::Undirected => &[],
            Self::Forward => &[false],
            Self::Reverse => &[true],
            Self::Both => &[false, true],
        }
    }
}

/// Заимствованное представление узла без данных предметной области.
#[derive(Clone, Copy, Debug)]
pub struct Node<'a> {
    /// Стабильный ключ внутри виджета; не зависит от позиции узла на экране.
    pub id: NodeId,
    pub position: Pos2,
    pub label: &'a str,
    pub accessibility_label: &'a str,
    /// Радиус в экранных точках, не зависит от масштаба камеры.
    pub radius: f32,
    pub color: Color32,
}

impl<'a> Node<'a> {
    pub fn new(id: NodeId, position: Pos2, label: &'a str) -> Self {
        Self {
            id,
            position,
            label,
            accessibility_label: label,
            radius: 6.0,
            color: Color32::LIGHT_BLUE,
        }
    }
}

/// Заимствованное представление ребра с предвычисленным путём.
#[derive(Clone, Copy, Debug)]
pub struct Edge<'a> {
    pub id: EdgeId,
    pub path: &'a EdgePath,
    pub color: Color32,
    pub direction: Direction,
    /// Топологическая петля; совпадение координат разных узлов петлёй не считается.
    pub is_loop: bool,
}

impl<'a> Edge<'a> {
    pub fn new(id: EdgeId, path: &'a EdgePath) -> Self {
        Self {
            id,
            path,
            color: Color32::LIGHT_GRAY,
            direction: Direction::Undirected,
            is_loop: false,
        }
    }
}
