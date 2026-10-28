use egui::pos2;

use super::*;

#[test]
fn routing_changes_geometry_without_changing_the_network_and_survives_station_moves() {
    for city in City::ALL {
        let mut metro = Metro::for_city(city);
        let route = metro.route(StationId(0), StationId(20)).unwrap();
        assert_eq!(metro.routing(), RoutingMode::Straight);
        for mode in [
            RoutingMode::Curved,
            RoutingMode::Orthogonal,
            RoutingMode::Octilinear,
            RoutingMode::Angular15,
            RoutingMode::Straight,
        ] {
            let before = metro.revision();
            assert!(metro.set_routing(mode).unwrap());
            assert_eq!(metro.routing(), mode);
            assert_eq!(metro.revision().topology(), before.topology());
            assert!(metro.revision().geometry() > before.geometry());
            let mut changed = Vec::new();
            assert_eq!(
                metro.geometry_changes(before, &mut |element| changed.push(element)),
                ChangeCoverage::Complete
            );
            assert_eq!(
                changed,
                (0..metro.edge_count())
                    .map(Selection::Edge)
                    .collect::<Vec<_>>()
            );
            let revision = metro.revision();
            assert!(!metro.set_routing(mode).unwrap());
            assert_eq!(metro.revision(), revision);
            let position = metro.stations[0].position + Vec2::new(37.0, 19.0);
            assert!(metro.move_station(StationId(0), position).unwrap());
            for edge in &metro.connections {
                let from = metro.stations[edge.from.0].position;
                let to = metro.stations[edge.to.0].position;
                assert_eq!(edge.path.points().first(), Some(&from));
                assert_eq!(edge.path.points().last(), Some(&to));
                assert!(metro.bounds().contains_rect(edge.path.bounds()));
                if mode == RoutingMode::Curved {
                    let pautinka::Curve::Quadratic(controls) = edge.path.curve() else {
                        panic!("Curved routing must use the library quadratic path");
                    };
                    assert_eq!(controls, [from, from.lerp(to, 0.5), to]);
                } else {
                    let step = match mode {
                        RoutingMode::Orthogonal => 90.0_f32,
                        RoutingMode::Octilinear => 45.0,
                        RoutingMode::Angular15 => 15.0,
                        RoutingMode::Straight => {
                            assert_eq!(edge.path.points(), [from, to]);
                            continue;
                        }
                        RoutingMode::Curved => unreachable!(),
                    };
                    for points in edge.path.points().windows(2) {
                        let delta = points[1] - points[0];
                        let angle = delta.y.atan2(delta.x).to_degrees() / step;
                        assert!((angle - angle.round()).abs() < 0.001);
                    }
                }
            }
            let after = metro.route(StationId(0), StationId(20)).unwrap();
            assert_eq!(after.stations, route.stations);
            assert_eq!(after.edges, route.edges);
            assert_eq!(after.minutes, route.minutes);
        }
    }
}

#[test]
fn moving_station_updates_connected_paths_bounds_and_geometry_history() {
    let mut metro = Metro::embedded();
    let before = metro.revision();
    let id = StationId(0);
    let old = metro.stations[id.0].position;
    let position = old + Vec2::new(-400.0, 20.0);
    let paths: Vec<_> = metro
        .connections
        .iter()
        .map(|edge| edge.path.points().to_vec())
        .collect();
    let old_route = metro.route(id, StationId(20)).unwrap();
    assert!(metro.move_station(id, position).unwrap());
    assert_eq!(metro.stations[id.0].position, position);
    assert_eq!(metro.revision().topology(), before.topology());
    assert!(metro.revision().geometry() > before.geometry());
    assert!(metro.bounds().contains(position));
    let mut changed = Vec::new();
    assert_eq!(
        metro.geometry_changes(before, &mut |element| changed.push(element)),
        ChangeCoverage::Complete
    );
    assert!(changed.contains(&Selection::Node(id.0)));
    for (index, edge) in metro.connections.iter().enumerate() {
        if edge.from == id || edge.to == id {
            assert_eq!(
                edge.path.points(),
                [
                    metro.stations[edge.from.0].position,
                    metro.stations[edge.to.0].position
                ]
            );
            assert!(changed.contains(&Selection::Edge(index)));
        } else {
            assert_eq!(edge.path.points(), paths[index]);
            assert!(!changed.contains(&Selection::Edge(index)));
        }
    }
    let new_route = metro.route(id, StationId(20)).unwrap();
    assert_eq!(old_route.stations, new_route.stations);
    assert_eq!(old_route.minutes, new_route.minutes);
    let revision = metro.revision();
    assert!(!metro.move_station(id, position).unwrap());
    assert!(!metro.move_station(StationId(usize::MAX), position).unwrap());
    for invalid in [
        pos2(f32::NAN, 0.0),
        pos2(f32::INFINITY, 0.0),
        pos2(f32::MAX, f32::MAX),
    ] {
        assert!(metro.move_station(id, invalid).is_err());
        assert_eq!(metro.stations[id.0].position, position);
        assert_eq!(metro.revision(), revision);
    }
    assert!(metro.move_station(id, old).unwrap());
    assert_eq!(metro.bounds(), station_bounds(&metro.stations));
    for (edge, points) in metro.connections.iter().zip(&paths) {
        assert_eq!(edge.path.points(), points);
    }
}

#[test]
fn failed_routing_change_keeps_paths_bounds_revision_and_mode() {
    let mut metro = Metro::embedded();
    let revision = metro.revision();
    let bounds = metro.bounds();
    let paths: Vec<_> = metro
        .connections
        .iter()
        .map(|edge| edge.path.points().to_vec())
        .collect();
    let station = metro.stations.last_mut().unwrap();
    let position = station.position;
    station.position.x = f32::NAN;
    assert!(metro.set_routing(RoutingMode::Curved).is_err());
    assert_eq!(metro.routing(), RoutingMode::Straight);
    assert_eq!(metro.revision(), revision);
    assert_eq!(metro.bounds(), bounds);
    for (edge, points) in metro.connections.iter().zip(paths) {
        assert_eq!(edge.path.points(), points);
    }
    metro.stations.last_mut().unwrap().position = position;
    assert!(metro.set_routing(RoutingMode::Curved).unwrap());
}

#[test]
fn rounded_geometry_survives_mode_changes_and_station_moves() {
    let mut metro = Metro::embedded();
    metro.set_routing(RoutingMode::Octilinear).unwrap();
    let sharp_lengths: Vec<_> = metro
        .connections
        .iter()
        .map(|edge| edge.path.length())
        .collect();
    let before = metro.revision();
    let radius = pautinka::CornerRadius::try_from(20.0).unwrap();
    assert!(metro.set_corner_radius(radius).unwrap());
    assert!(metro.revision().geometry() > before.geometry());
    assert_eq!(metro.revision().topology(), before.topology());
    assert!(
        metro
            .connections
            .iter()
            .zip(sharp_lengths)
            .any(|(edge, sharp)| edge.path.length() < sharp - 0.01)
    );
    assert!(!metro.set_corner_radius(radius).unwrap());
    metro.set_routing(RoutingMode::Straight).unwrap();
    assert_eq!(metro.corner_radius(), radius);
    metro.set_routing(RoutingMode::Angular15).unwrap();
    metro
        .move_station(
            StationId(0),
            metro.stations[0].position + Vec2::new(37.0, 19.0),
        )
        .unwrap();
    for edge in &metro.connections {
        let expected = connection_path(
            metro.routing(),
            radius,
            metro.stations[edge.from.0].position,
            metro.stations[edge.to.0].position,
        )
        .unwrap();
        assert_eq!(edge.path.points(), expected.points());
        assert!(metro.bounds().contains_rect(edge.path.bounds()));
    }
    metro
        .set_corner_radius(pautinka::CornerRadius::default())
        .unwrap();
    for edge in &metro.connections {
        let expected = metro
            .routing()
            .route(
                RouteEndpoints::Between {
                    from: metro.stations[edge.from.0].position,
                    to: metro.stations[edge.to.0].position,
                },
                0.0,
                0,
            )
            .unwrap();
        assert_eq!(edge.path.points(), expected.points());
    }
}
