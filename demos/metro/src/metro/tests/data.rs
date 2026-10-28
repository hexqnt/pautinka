use super::*;

fn station(metro: &Metro, name: &str, line: Line) -> StationId {
    StationId(
        metro
            .stations
            .iter()
            .position(|station| station.name == name && station.line == line)
            .unwrap_or_else(|| panic!("Missing station: {name}, {line:?}")),
    )
}

fn direct_connection(metro: &Metro, from: StationId, to: StationId) -> bool {
    metro.adjacency[from.0]
        .iter()
        .any(|&(neighbor, _)| neighbor == to)
}

#[test]
fn petersburg_names_lines_and_direct_connections_match_the_audit() {
    let metro = Metro::embedded();
    for (line, names) in [
        (
            Line::Green,
            &["Маяковская", "Площадь Александра Невского"][..],
        ),
        (
            Line::Purple,
            &[
                "Комендантский проспект",
                "Старая Деревня",
                "Крестовский остров",
            ],
        ),
        (
            Line::Blue,
            &[
                "Чёрная речка",
                "Сенная площадь",
                "Технологический институт",
                "Московские ворота",
                "Звёздная",
            ],
        ),
        (
            Line::Red,
            &[
                "Площадь Мужества",
                "Владимирская",
                "Технологический институт",
                "Кировский завод",
                "Проспект Ветеранов",
            ],
        ),
        (
            Line::Orange,
            &[
                "Лиговский проспект",
                "Площадь Александра Невского",
                "Проспект Большевиков",
                "Горный институт",
            ],
        ),
        (Line::Brown, &["Путиловская", "Юго-Западная"]),
    ] {
        for &name in names {
            station(&metro, name, line);
        }
    }
    assert!(
        metro
            .stations
            .iter()
            .all(|station| station.name != "Площадь Кати")
    );
    assert_eq!(Line::Orange.name(), "Лахтинско-Правобережная");
    for ((from, from_line), (to, to_line)) in [
        (
            ("Маяковская", Line::Green),
            ("Площадь Александра Невского", Line::Green),
        ),
        (("Садовая", Line::Purple), ("Спасская", Line::Orange)),
        (
            ("Горный институт", Line::Orange),
            ("Спасская", Line::Orange),
        ),
        (("Юго-Западная", Line::Brown), ("Путиловская", Line::Brown)),
        (("Путиловская", Line::Brown), ("Кировский завод", Line::Red)),
    ] {
        let from = station(&metro, from, from_line);
        let to = station(&metro, to, to_line);
        assert!(direct_connection(&metro, from, to));
        let route = metro.route(from, to).unwrap();
        assert_eq!(route.stations, [from, to]);
        assert_eq!(route.transfers, usize::from(from_line != to_line));
    }
}

#[test]
fn moscow_snapshot_contains_only_direct_transfers_and_restored_solntsevskaya_terminus() {
    let metro = Metro::for_city(City::Moscow);
    station(&metro, "Народное Ополчение", Line::Moscow11);
    assert!(
        metro
            .stations
            .iter()
            .all(|station| station.name != "Карамышевская")
    );
    for ((from, from_line), (to, to_line), direct) in [
        (
            ("Охотный Ряд", Line::Moscow1),
            ("Площадь Революции", Line::Moscow3),
            false,
        ),
        (
            ("Александровский сад", Line::Moscow4),
            ("Боровицкая", Line::Moscow9),
            false,
        ),
        (
            ("Красногвардейская", Line::Moscow2),
            ("Зябликово", Line::Moscow10),
            true,
        ),
        (
            ("Деловой центр", Line::Moscow8A),
            ("Парк Победы", Line::Moscow8A),
            true,
        ),
        (
            ("Деловой центр", Line::Moscow8A),
            ("Деловой центр", Line::Moscow11),
            true,
        ),
        (
            ("Деловой центр", Line::Moscow8A),
            ("Выставочная", Line::Moscow4),
            true,
        ),
        (
            ("Парк Победы", Line::Moscow8A),
            ("Шелепиха", Line::Moscow11),
            false,
        ),
    ] {
        let from = station(&metro, from, from_line);
        let to = station(&metro, to, to_line);
        assert_eq!(direct_connection(&metro, from, to), direct);
        let route = metro.route(from, to).unwrap();
        assert_eq!(route.edges.len() == 1, direct);
    }
    let route = metro
        .route(
            station(&metro, "Охотный Ряд", Line::Moscow1),
            station(&metro, "Площадь Революции", Line::Moscow3),
        )
        .unwrap();
    assert_eq!(metro.stations[route.stations[1].0].name, "Театральная");
    assert_eq!((route.minutes, route.transfers), (8, 2));
    for name in ["Авиамоторная", "Лефортово", "Электрозаводская"]
    {
        station(&metro, name, Line::Moscow15);
    }
}

#[test]
fn closed_stations_allow_travel_through_but_not_boarding_or_alighting() {
    for (city, line, before, closed, after) in [
        (
            City::SaintPetersburg,
            Line::Blue,
            "Технологический институт",
            "Фрунзенская",
            "Московские ворота",
        ),
        (
            City::SaintPetersburg,
            Line::Blue,
            "Электросила",
            "Парк Победы",
            "Московская",
        ),
        (
            City::Moscow,
            Line::Moscow3,
            "Киевская",
            "Смоленская",
            "Арбатская",
        ),
        (
            City::Moscow,
            Line::Moscow6,
            "Алексеевская",
            "Рижская",
            "Проспект Мира",
        ),
    ] {
        let metro = Metro::for_city(city);
        let before = station(&metro, before, line);
        let closed = station(&metro, closed, line);
        let after = station(&metro, after, line);
        assert_eq!(metro.stations[closed.0].access, StationAccess::TransitOnly);
        assert!(metro.route(before, closed).is_none());
        assert!(metro.route(closed, after).is_none());
        assert!(metro.route(closed, closed).is_none());
        assert_eq!(
            metro.route(before, after).unwrap().stations,
            [before, closed, after]
        );
        assert_eq!(
            metro.route(after, before).unwrap().stations,
            [after, closed, before]
        );
    }
}

#[test]
fn moscow_snapshot_has_only_active_stations_and_preserves_line_connections() {
    let metro = Metro::for_city(City::Moscow);
    assert!(metro.stations.iter().all(|station| {
        !["Суворовская", "Физтех", "Внуково", "Варшавская"].contains(&station.name.as_str())
    }));
    let route = metro
        .route(
            station(&metro, "Охотный Ряд", Line::Moscow1),
            station(&metro, "Театральная", Line::Moscow2),
        )
        .unwrap();
    assert_eq!((route.minutes, route.transfers), (4, 1));
    let route = metro
        .route(
            station(&metro, "Крылатское", Line::Moscow3),
            station(&metro, "Молодёжная", Line::Moscow3),
        )
        .unwrap();
    assert_eq!((route.minutes, route.transfers), (3, 0));
    let ring = station(&metro, "Проспект Мира", Line::Moscow5);
    assert_eq!(
        metro.adjacency[ring.0]
            .iter()
            .filter(|&&(other, _)| metro.stations[other.0].line == Line::Moscow5)
            .count(),
        2
    );
}
