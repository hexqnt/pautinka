mod city;
mod line;
mod load;
mod route;

use egui::{Color32, Pos2, Rect, Vec2};
use pautinka::{
    ChangeCoverage, CornerRadius, EdgePath, GeometryChanges, PathError, RouteEndpoints,
    RoutingMode,
    egui::{Edge, EdgeId, Graph, GraphId, GraphRevision, Node, NodeId, Selection},
};

use crate::i18n::{Language, Text};

pub use city::City;
pub use line::Line;

/// Пассажирская доступность станции, независимая от наличия путей.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationAccess {
    #[default]
    Open,
    /// Разрешён только проезд по своей линии, без посадки, высадки и пересадки.
    TransitOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StationId(pub usize);

pub struct Station {
    pub name: String,
    pub position: Pos2,
    pub line: Line,
    pub access: StationAccess,
    pub label_offset: Vec2,
    pub label_align: egui::Align2,
}

pub struct Connection {
    pub from: StationId,
    pub to: StationId,
    pub minutes: u16,
    pub path: EdgePath,
}

pub struct Route {
    pub stations: Vec<StationId>,
    pub edges: Vec<usize>,
    pub minutes: u32,
    pub transfers: usize,
}

pub struct Metro {
    pub(crate) language: Language,
    id: GraphId,
    pub stations: Vec<Station>,
    pub connections: Vec<Connection>,
    adjacency: Vec<Vec<(StationId, usize)>>,
    bounds: Rect,
    revision: GraphRevision,
    changes: GeometryChanges,
    pending_paths: Vec<EdgePath>,
    routing: RoutingMode,
    corner_radius: CornerRadius,
}

impl Metro {
    #[must_use]
    pub const fn routing(&self) -> RoutingMode {
        self.routing
    }

    /// Перестраивает пути всех рёбер, сохраняя станции и маршрут поездки.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку без изменения графа, если новый путь не представим.
    pub fn set_routing(&mut self, mode: RoutingMode) -> Result<bool, PathError> {
        self.set_geometry(mode, self.corner_radius)
    }

    pub const fn corner_radius(&self) -> CornerRadius {
        self.corner_radius
    }

    /// Перестраивает геометрию с новым радиусом; при ошибке сохраняет предыдущие пути и настройку.
    pub fn set_corner_radius(&mut self, radius: CornerRadius) -> Result<bool, PathError> {
        self.set_geometry(self.routing, radius)
    }

    pub(crate) fn set_geometry(
        &mut self,
        mode: RoutingMode,
        radius: CornerRadius,
    ) -> Result<bool, PathError> {
        if self.routing == mode && self.corner_radius == radius {
            return Ok(false);
        }
        let mut paths = Vec::with_capacity(self.connections.len());
        for edge in &self.connections {
            paths.push(connection_path(
                mode,
                radius,
                self.stations[edge.from.0].position,
                self.stations[edge.to.0].position,
            )?);
        }
        for (edge, path) in self.connections.iter_mut().zip(paths) {
            edge.path = path;
        }
        self.routing = mode;
        self.corner_radius = radius;
        let before = self.revision;
        self.revision = before.next_geometry();
        self.changes.record(
            before,
            self.revision,
            (0..self.connections.len()).map(Selection::Edge),
        );
        self.update_bounds();
        Ok(true)
    }

    /// Перемещает станцию и обновляет только связанные пути и журнал геометрии. Возвращает `false`, если станция отсутствует или позиция не изменилась.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку без изменения графа, если новая позиция или длина связанного пути не представима.
    pub fn move_station(&mut self, id: StationId, position: Pos2) -> Result<bool, PathError> {
        let Some(station) = self.stations.get(id.0) else {
            return Ok(false);
        };
        if !position.is_finite() {
            return Err(PathError::NonFiniteCoordinate);
        }
        if station.position == position {
            return Ok(false);
        }
        // Буфер выделен при загрузке по максимальной степени вершины и переиспользуется во время жеста.
        self.pending_paths.clear();
        for &(_, index) in &self.adjacency[id.0] {
            let edge = &self.connections[index];
            let from = if edge.from == id {
                position
            } else {
                self.stations[edge.from.0].position
            };
            let to = if edge.to == id {
                position
            } else {
                self.stations[edge.to.0].position
            };
            self.pending_paths
                .push(connection_path(self.routing, self.corner_radius, from, to)?);
        }
        self.stations[id.0].position = position;
        for (&(_, index), path) in self.adjacency[id.0]
            .iter()
            .zip(self.pending_paths.drain(..))
        {
            self.connections[index].path = path;
        }
        let before = self.revision;
        self.revision = before.next_geometry();
        self.changes.record(
            before,
            self.revision,
            std::iter::once(Selection::Node(id.0)).chain(
                self.adjacency[id.0]
                    .iter()
                    .map(|&(_, index)| Selection::Edge(index)),
            ),
        );
        self.update_bounds();
        Ok(true)
    }

    fn update_bounds(&mut self) {
        self.bounds = self
            .connections
            .iter()
            .fold(station_bounds(&self.stations), |bounds, edge| {
                bounds.union(edge.path.bounds())
            });
    }

    #[must_use]
    pub fn is_transfer(&self, edge: usize) -> bool {
        let edge = &self.connections[edge];
        self.stations[edge.from.0].line != self.stations[edge.to.0].line
    }
}

impl Graph for Metro {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        self.revision
    }
    fn geometry_changes(
        &self,
        since: GraphRevision,
        visit: &mut dyn FnMut(Selection),
    ) -> ChangeCoverage {
        self.changes.visit(since, self.revision, visit)
    }
    fn node_count(&self) -> usize {
        self.stations.len()
    }
    fn edge_count(&self) -> usize {
        self.connections.len()
    }
    fn bounds(&self) -> Rect {
        self.bounds
    }
    fn node(&self, index: usize) -> Node<'_> {
        let station = &self.stations[index];
        let mut node = Node::new(NodeId::new(index as u64), station.position, &station.name);
        node.color = station.line.color();
        node.radius = 4.5;
        node
    }
    fn edge(&self, index: usize) -> Edge<'_> {
        let edge = &self.connections[index];
        let mut result = Edge::new(EdgeId::new(index as u64), &edge.path);
        result.color = if self.is_transfer(index) {
            Color32::from_rgb(133, 143, 157)
        } else {
            self.stations[edge.from.0].line.color()
        };
        result
    }
    fn node_tooltip(&self, ui: &mut egui::Ui, index: usize) {
        let language = self.language;
        let station = &self.stations[index];
        ui.strong(&station.name);
        ui.colored_label(station.line.color(), language.line_name(station.line));
        ui.weak(if station.access == StationAccess::Open {
            language.text(Text::SelectStation)
        } else {
            language.text(Text::StationClosed)
        });
    }
    fn edge_tooltip(&self, ui: &mut egui::Ui, index: usize) {
        let language = self.language;
        let edge = &self.connections[index];
        ui.label(format!(
            "{} — {}",
            self.stations[edge.from.0].name, self.stations[edge.to.0].name
        ));
        ui.label(format!(
            "{} {} · {}",
            edge.minutes,
            language.text(Text::Minutes),
            language.text(Text::ExampleData)
        ));
    }
}

fn connection_path(
    mode: RoutingMode,
    radius: CornerRadius,
    from: Pos2,
    to: Pos2,
) -> Result<EdgePath, PathError> {
    // Загрузчик объединяет параллельные связи, поэтому смещение дорожки всегда нулевое.
    // Ломаные хранят точки в принадлежащем EdgePath буфере; его аллокация нужна и при перетаскивании.
    mode.route(RouteEndpoints::Between { from, to }, 0.0, 0)?
        .with_corner_radius(radius)
}

fn station_bounds(stations: &[Station]) -> Rect {
    stations
        .iter()
        .fold(Rect::NOTHING, |bounds, station| {
            bounds.union(Rect::from_center_size(station.position, Vec2::splat(12.0)))
        })
        .expand2(Vec2::new(195.0, 45.0))
}

#[cfg(test)]
mod tests;
