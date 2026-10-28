use super::{Metro, Route, StationAccess, StationId};

impl Metro {
    #[must_use]
    pub fn route(&self, from: StationId, to: StationId) -> Option<Route> {
        if self.stations.get(from.0)?.access != StationAccess::Open
            || self.stations.get(to.0)?.access != StationAccess::Open
        {
            return None;
        }
        if from == to {
            return Some(Route {
                stations: vec![from],
                edges: Vec::new(),
                minutes: 0,
                transfers: 0,
            });
        }
        // Линейный выбор минимума сканирует компактные расстояния и флаги отдельно от истории пути.
        let mut distances = vec![u32::MAX; self.stations.len()];
        let mut previous = vec![None; self.stations.len()];
        let mut visited = vec![false; self.stations.len()];
        distances[from.0] = 0;
        loop {
            let (current, distance) = distances
                .iter()
                .zip(&visited)
                .enumerate()
                .filter(|&(_, (_, visited))| !visited)
                .min_by_key(|&(_, (distance, _))| distance)
                .map(|(index, (&distance, _))| (index, distance))?;
            if distance == u32::MAX {
                return None;
            }
            if current == to.0 {
                break;
            }
            visited[current] = true;
            for &(neighbor, edge) in &self.adjacency[current] {
                if self.is_transfer(edge)
                    && (self.stations[current].access != StationAccess::Open
                        || self.stations[neighbor.0].access != StationAccess::Open)
                {
                    continue;
                }
                let candidate = distance + u32::from(self.connections[edge].minutes);
                if candidate < distances[neighbor.0] {
                    distances[neighbor.0] = candidate;
                    previous[neighbor.0] = Some((StationId(current), edge));
                }
            }
        }
        let mut stations = vec![to];
        let mut edges = Vec::new();
        let mut current = to;
        while current != from {
            let (parent, edge) = previous[current.0]?;
            stations.push(parent);
            edges.push(edge);
            current = parent;
        }
        stations.reverse();
        edges.reverse();
        let transfers = edges.iter().filter(|&&i| self.is_transfer(i)).count();
        Some(Route {
            stations,
            edges,
            minutes: distances[to.0],
            transfers,
        })
    }
}
