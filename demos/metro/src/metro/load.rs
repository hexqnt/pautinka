use std::{
    collections::{BTreeMap, btree_map::Entry},
    num::NonZeroU16,
};

use egui::{Vec2, pos2};
use pautinka::{
    EdgePath, GeometryChanges, RoutingMode,
    egui::{GraphId, GraphRevision},
};
use serde::Deserialize;

use super::{City, Connection, Line, Metro, Station, StationAccess, StationId, station_bounds};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Shift {
    Left,
    Right,
    Up,
    Down,
}

impl Shift {
    const fn label(self) -> (Vec2, egui::Align2) {
        match self {
            Self::Left => (Vec2::new(-10.0, 0.0), egui::Align2::RIGHT_CENTER),
            Self::Right => (Vec2::new(10.0, 0.0), egui::Align2::LEFT_CENTER),
            Self::Up => (Vec2::new(0.0, -12.0), egui::Align2::CENTER_BOTTOM),
            Self::Down => (Vec2::new(0.0, 12.0), egui::Align2::CENTER_TOP),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawStation {
    name: String,
    x: i16,
    y: i16,
    adj: Vec<String>,
    dist: Vec<NonZeroU16>,
    line: Line,
    #[serde(default)]
    access: StationAccess,
    shift: Shift,
    dx: i16,
    dy: i16,
}

impl Metro {
    /// Загружает встроенную карту Санкт-Петербурга для демо и воспроизводимых измерений.
    ///
    /// # Panics
    ///
    /// Если встроенный файл содержит некорректную карту.
    #[must_use]
    pub fn embedded() -> Self {
        Self::for_city(City::default())
    }

    /// Загружает встроенную карту выбранного города.
    ///
    /// # Panics
    ///
    /// Если встроенный файл содержит некорректную карту.
    #[must_use]
    pub fn for_city(city: City) -> Self {
        Self::parse(city.data()).expect("Embedded metro data must be valid")
    }

    /// Загружает JSON-карту и подготавливает геометрию графа.
    ///
    /// # Errors
    ///
    /// Возвращает ошибку при неверном формате, пустой карте, некорректных станциях, неизвестных ссылках или противоречивых связях.
    pub fn parse(data: &str) -> Result<Self, String> {
        let raw: BTreeMap<String, RawStation> =
            serde_json::from_str(data).map_err(|error| error.to_string())?;
        if raw.is_empty() {
            return Err("Metro must contain stations".into());
        }
        for (key, station) in &raw {
            if station.name.trim().is_empty() || station.adj.len() != station.dist.len() {
                return Err(format!("Invalid station: {key}"));
            }
        }
        // Ключи и станции перемещаются из JSON-модели без копирования строк; порядок ID остаётся лексикографическим.
        let (ids, raw): (BTreeMap<_, _>, Vec<_>) = raw
            .into_iter()
            .enumerate()
            .map(|(index, (key, station))| ((key, StationId(index)), station))
            .unzip();
        let mut stations = Vec::with_capacity(raw.len());
        let mut weights = BTreeMap::new();
        let mut pending = Vec::new();
        for (index, station) in raw.into_iter().enumerate() {
            let from = StationId(index);
            for (target, minutes) in station.adj.iter().zip(&station.dist) {
                let to = *ids
                    .get(target)
                    .ok_or_else(|| format!("Unknown station: {target}"))?;
                if from == to {
                    return Err(format!("Self connection: {target}"));
                }
                let pair = if from < to { (from, to) } else { (to, from) };
                match weights.entry(pair) {
                    Entry::Occupied(entry) => {
                        if entry.get() != minutes {
                            return Err(format!(
                                "Conflicting connection: {} -> {target}",
                                station.name
                            ));
                        }
                    }
                    Entry::Vacant(entry) => {
                        entry.insert(*minutes);
                        pending.push((from, to, *minutes));
                    }
                }
            }
            let (offset, align) = station.shift.label();
            stations.push(Station {
                name: station.name,
                position: pos2(f32::from(station.x), f32::from(station.y)) * 2.0,
                line: station.line,
                access: station.access,
                label_offset: (offset + Vec2::new(f32::from(station.dx), f32::from(station.dy)))
                    * 2.0,
                label_align: align,
            });
        }
        let mut adjacency = vec![Vec::new(); stations.len()];
        let mut connections = Vec::with_capacity(pending.len());
        for (index, (from, to, minutes)) in pending.into_iter().enumerate() {
            let path = EdgePath::try_line(stations[from.0].position, stations[to.0].position)
                .map_err(|error| error.to_string())?;
            connections.push(Connection {
                from,
                to,
                minutes: minutes.get(),
                path,
            });
            adjacency[from.0].push((to, index));
            adjacency[to.0].push((from, index));
        }
        let bounds = station_bounds(&stations);
        let pending_paths = Vec::with_capacity(adjacency.iter().map(Vec::len).max().unwrap_or(0));
        Ok(Self {
            language: crate::Language::Russian,
            id: GraphId::fresh(),
            stations,
            connections,
            adjacency,
            bounds,
            revision: GraphRevision::default(),
            changes: GeometryChanges::default(),
            pending_paths,
            routing: RoutingMode::Straight,
            corner_radius: pautinka::CornerRadius::default(),
        })
    }
}
