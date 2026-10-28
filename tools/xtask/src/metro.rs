use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use time::{
    Date,
    macros::{date, format_description},
};

const SNAPSHOT: Date = date!(2021 - 04 - 01);
const COORDINATE_TOLERANCE: f64 = 0.00001;
const RIDE_MINUTES: u8 = 3;
const TRANSFER_MINUTES: u8 = 4;
const SKIPPED: [(f64, f64); 2] = [(55.7919, 37.4006), (55.7802, 37.6204)];
const TRANSFERS: &[(&str, &str)] = &[
    ("msk_005_1SL", "msk_091_5KL"),    // Комсомольская — Комсомольская
    ("msk_007_1SL", "msk_106_6KRL"),   // Чистые пруды — Тургеневская
    ("msk_007_1SL", "msk_198_10LDL"),  // Чистые пруды — Сретенский бульвар
    ("msk_008_1SL", "msk_132_7TKL"),   // Лубянка — Кузнецкий Мост
    ("msk_009_1SL", "msk_037_2ZL"),    // Охотный Ряд — Театральная
    ("msk_010_1SL", "msk_062_3APL"),   // Библиотека имени Ленина — Арбатская
    ("msk_010_1SL", "msk_084_4FL"),    // Библиотека имени Ленина — Александровский сад
    ("msk_010_1SL", "msk_175_9STL"),   // Библиотека имени Ленина — Боровицкая
    ("msk_012_1SL", "msk_085_5KL"),    // Парк культуры — Парк культуры
    ("msk_033_2ZL", "msk_218_11BKL"),  // Динамо — Петровский парк
    ("msk_034_2ZL", "msk_094_5KL"),    // Белорусская — Белорусская
    ("msk_036_2ZL", "msk_131_7TKL"),   // Тверская — Пушкинская
    ("msk_036_2ZL", "msk_174_9STL"),   // Тверская — Чеховская
    ("msk_037_2ZL", "msk_063_3APL"),   // Театральная — Площадь Революции
    ("msk_038_2ZL", "msk_108_6KRL"),   // Новокузнецкая — Третьяковская
    ("msk_038_2ZL", "msk_156_8KL"),    // Новокузнецкая — Третьяковская
    ("msk_039_2ZL", "msk_088_5KL"),    // Павелецкая — Павелецкая
    ("msk_057_3APL", "msk_072_4FL"),   // Кунцевская — Кунцевская
    ("msk_059_3APL", "msk_154_8SL"),   // Парк Победы — Парк Победы
    ("msk_060_3APL", "msk_081_4FL"),   // Киевская — Киевская
    ("msk_060_3APL", "msk_096_5KL"),   // Киевская — Киевская
    ("msk_062_3APL", "msk_084_4FL"),   // Арбатская — Александровский сад
    ("msk_062_3APL", "msk_175_9STL"),  // Арбатская — Боровицкая
    ("msk_064_3APL", "msk_090_5KL"),   // Курская — Курская
    ("msk_064_3APL", "msk_199_10LDL"), // Курская — Чкаловская
    ("msk_066_3APL", "msk_240_15NL"),  // Электрозаводская — Электрозаводская
    ("msk_080_4FL", "msk_212_11BKL"),  // Выставочная — Деловой центр
    ("msk_081_4FL", "msk_096_5KL"),    // Киевская — Киевская
    ("msk_086_5KL", "msk_109_6KRL"),   // Октябрьская — Октябрьская
    ("msk_087_5KL", "msk_177_9STL"),   // Добрынинская — Серпуховская
    ("msk_089_5KL", "msk_134_7TKL"),   // Таганская — Таганская
    ("msk_089_5KL", "msk_157_8KL"),    // Таганская — Марксистская
    ("msk_090_5KL", "msk_199_10LDL"),  // Курская — Чкаловская
    ("msk_092_5KL", "msk_104_6KRL"),   // Проспект Мира — Проспект Мира
    ("msk_093_5KL", "msk_172_9STL"),   // Новослободская — Менделеевская
    ("msk_095_5KL", "msk_130_7TKL"),   // Краснопресненская — Баррикадная
    ("msk_106_6KRL", "msk_198_10LDL"), // Тургеневская — Сретенский бульвар
    ("msk_107_6KRL", "msk_133_7TKL"),  // Китай-город — Китай-город
    ("msk_108_6KRL", "msk_156_8KL"),   // Третьяковская — Третьяковская
    ("msk_120_6KRL", "msk_223_12BL"),  // Новоясеневская — Битцевский парк
    ("msk_127_7TKL", "msk_216_11BKL"), // Полежаевская — Хорошёвская
    ("msk_131_7TKL", "msk_174_9STL"),  // Пушкинская — Чеховская
    ("msk_134_7TKL", "msk_157_8KL"),   // Таганская — Марксистская
    ("msk_135_7TKL", "msk_201_10LDL"), // Пролетарская — Крестьянская застава
    ("msk_141_7TKL", "msk_233_15NL"),  // Лермонтовский проспект — Косино
    ("msk_158_8KL", "msk_200_10LDL"),  // Площадь Ильича — Римская
    ("msk_159_8KL", "msk_238_15NL"),   // Авиамоторная — Авиамоторная
    ("msk_168_9STL", "msk_192_10LDL"), // Петровско-Разумовская — Петровско-Разумовская
    ("msk_171_9STL", "msk_219_11BKL"), // Савёловская — Савёловская
    ("msk_173_9STL", "msk_197_10LDL"), // Цветной бульвар — Трубная
    ("msk_188_9STL", "msk_225_12BL"),  // Бульвар Дмитрия Донского — Улица Старокачаловская
    ("msk_048_2ZL", "msk_211_10LDL"),  // Красногвардейская — Зябликово
    ("msk_080_4FL", "msk_155_8SL"),    // Выставочная — Деловой центр
    ("msk_155_8SL", "msk_212_11BKL"),  // Деловой центр — Деловой центр
];

fn optional_date<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Date>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|value| {
            Date::parse(&value, format_description!("[year]-[month]-[day]"))
                .map_err(serde::de::Error::custom)
        })
        .transpose()
}

#[derive(Deserialize)]
#[serde(try_from = "RawPeriod")]
struct Period {
    since: Date,
    until: Date,
}

#[derive(Deserialize)]
struct RawPeriod {
    #[serde(default, deserialize_with = "optional_date")]
    since: Option<Date>,
    #[serde(default, rename = "to", deserialize_with = "optional_date")]
    until: Option<Date>,
}

impl TryFrom<RawPeriod> for Period {
    type Error = anyhow::Error;

    fn try_from(raw: RawPeriod) -> Result<Self> {
        let since = raw.since.unwrap_or(Date::MIN);
        let until = raw.until.unwrap_or(Date::MAX);
        ensure!(since < until, "Invalid period: {since}..{until}");
        Ok(Self { since, until })
    }
}

impl Period {
    fn active(&self) -> bool {
        self.since <= SNAPSHOT && SNAPSHOT < self.until
    }
}

#[derive(Deserialize)]
struct Line {
    name: String,
    #[serde(flatten)]
    period: Period,
}

#[derive(Deserialize)]
struct Coordinates {
    lat: f64,
    lon: f64,
}

#[derive(Deserialize)]
struct Station {
    name: String,
    dates: Vec<Period>,
    lines: Vec<Line>,
    #[serde(default)]
    closures: Vec<Period>,
    coords: Coordinates,
}

#[derive(Deserialize)]
struct Section {
    line: String,
    coords: CoordinatePairs,
    #[serde(flatten)]
    period: Period,
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct CoordinatePairs(Vec<(f64, f64)>);

impl TryFrom<String> for CoordinatePairs {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        let mut values = value.split(',').map(str::parse::<f64>);
        let mut pairs = Vec::new();
        while let Some(latitude) = values.next() {
            let latitude = latitude?;
            let longitude = values.next().context("Incomplete coordinate pair")??;
            ensure!(
                latitude.is_finite() && longitude.is_finite(),
                "Non-finite coordinate"
            );
            pairs.push((latitude, longitude));
        }
        Ok(Self(pairs))
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct Node {
    name: String,
    x: i64,
    y: i64,
    adj: Vec<String>,
    dist: Vec<u8>,
    line: String,
    shift: &'static str,
    dx: u8,
    dy: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    access: Option<&'static str>,
    #[serde(skip)]
    coordinates: (f64, f64),
}

type Map = IndexMap<String, Node>;

fn connect(stations: &mut Map, source: &str, target: &str, minutes: u8) {
    let station = stations.get_mut(source).expect("source was resolved");
    if source != target && !station.adj.iter().any(|adj| adj == target) {
        station.adj.push(target.to_owned());
        station.dist.push(minutes);
    }
}

fn build(data: &Path) -> Result<Map> {
    let input: Vec<Station> = crate::io::read_json(&data.join("stations-msk.json"))?;
    let sections: Vec<Section> = crate::io::read_json(&data.join("sections-msk.json"))?;
    let mut stations = Map::new();
    for (index, station) in input.into_iter().enumerate() {
        if !station.dates.iter().any(Period::active) {
            continue;
        }
        for line in station
            .lines
            .into_iter()
            .filter(|line| line.period.active())
        {
            stations.insert(
                format!("msk_{index:03}_{}", line.name),
                Node {
                    name: station.name.clone(),
                    x: ((station.coords.lon - 37.6) * 4200.0).round_ties_even() as i64,
                    y: ((55.75 - station.coords.lat) * 7500.0).round_ties_even() as i64,
                    adj: Vec::new(),
                    dist: Vec::new(),
                    line: line.name,
                    shift: "right",
                    dx: 0,
                    dy: 0,
                    access: station
                        .closures
                        .iter()
                        .any(Period::active)
                        .then_some("transit_only"),
                    coordinates: (station.coords.lat, station.coords.lon),
                },
            );
        }
    }
    for section in sections
        .into_iter()
        .filter(|section| section.period.active())
    {
        let mut previous: Option<String> = None;
        for (latitude, longitude) in section.coords.0 {
            if SKIPPED.contains(&(latitude, longitude)) {
                continue;
            }
            let line = if section.line == "8SL" && (latitude, longitude) == (55.7574, 37.524) {
                "11BKL"
            } else {
                &section.line
            };
            let mut matches = stations.iter().filter(|(_, station)| {
                station.line == line
                    && (station.coordinates.0 - latitude).abs() < COORDINATE_TOLERANCE
                    && (station.coordinates.1 - longitude).abs() < COORDINATE_TOLERANCE
            });
            let Some((key, _)) = matches.next() else {
                bail!("No station for {line} at ({latitude}, {longitude})");
            };
            ensure!(
                matches.next().is_none(),
                "Ambiguous station for {line} at ({latitude}, {longitude})"
            );
            let key = key.clone();
            if let Some(source) = &previous {
                connect(&mut stations, source, &key, RIDE_MINUTES);
            }
            previous = Some(key);
        }
    }
    let used: std::collections::HashSet<_> = stations
        .iter()
        .filter(|(_, station)| !station.adj.is_empty())
        .map(|(key, _)| key.clone())
        .chain(
            stations
                .values()
                .flat_map(|station| station.adj.iter().cloned()),
        )
        .collect();
    stations.retain(|key, _| used.contains(key));
    for &(source, target) in TRANSFERS {
        let from = stations
            .get(source)
            .ok_or_else(|| anyhow::anyhow!("Unknown transfer station: {source}"))?;
        let to = stations
            .get(target)
            .ok_or_else(|| anyhow::anyhow!("Unknown transfer station: {target}"))?;
        ensure!(
            from.line != to.line,
            "Transfer within one line: {source} -> {target}"
        );
        connect(&mut stations, source, target, TRANSFER_MINUTES);
    }
    Ok(stations)
}

pub fn write(data: &Path) -> Result<()> {
    crate::io::write_json(&data.join("msk.json"), &build(data)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproduces_checked_in_map() {
        let data = crate::root().unwrap().join("demos/metro/data");
        let output = tempfile::tempdir().unwrap();
        let path = output.path().join("msk.json");
        crate::io::write_json(&path, &build(&data).unwrap()).unwrap();
        assert_eq!(
            std::fs::read(path).unwrap(),
            std::fs::read(data.join("msk.json")).unwrap()
        );
    }

    #[test]
    fn rejects_invalid_input() {
        for value in [
            r#"{"since":"2021-02-30"}"#,
            r#"{"since":"2022-01-01","to":"2021-01-01"}"#,
        ] {
            assert!(serde_json::from_str::<Period>(value).is_err());
        }
        for value in ["1,2,3", "NaN,2", "1,x"] {
            assert!(CoordinatePairs::try_from(value.to_owned()).is_err());
        }
    }
}
