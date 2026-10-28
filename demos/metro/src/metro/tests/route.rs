use super::*;

#[test]
fn embedded_networks_are_connected_and_routes_are_reversible() {
    for (city, count) in [(City::SaintPetersburg, 75), (City::Moscow, 238)] {
        let metro = Metro::for_city(city);
        assert_eq!(metro.stations.len(), count);
        for &line in city.lines() {
            assert!(metro.stations.iter().any(|station| station.line == line));
        }
        for (index, station) in metro.stations.iter().enumerate() {
            assert!(city.lines().contains(&station.line));
            if station.access != StationAccess::Open {
                continue;
            }
            let route = metro.route(StationId(0), StationId(index)).unwrap();
            let reverse = metro.route(StationId(index), StationId(0)).unwrap();
            assert_eq!(route.minutes, reverse.minutes);
            assert_eq!(
                route.minutes,
                route
                    .edges
                    .iter()
                    .map(|&edge| u32::from(metro.connections[edge].minutes))
                    .sum::<u32>()
            );
            assert_eq!(route.stations.len(), route.edges.len() + 1);
            for (pair, &edge) in route.stations.windows(2).zip(&route.edges) {
                let edge = &metro.connections[edge];
                assert!(
                    (edge.from == pair[0] && edge.to == pair[1])
                        || (edge.to == pair[0] && edge.from == pair[1])
                );
            }
        }
    }
}

#[test]
fn shortest_route_uses_weights_and_transfers() {
    let metro = Metro::parse(r#"{
        "a":{"Name":"A","X":0,"Y":0,"Adj":["b","c"],"Dist":[9,2],"Line":"red","Shift":"left","Dx":0,"Dy":0},
        "b":{"Name":"B","X":10,"Y":0,"Adj":[],"Dist":[],"Line":"blue","Shift":"left","Dx":0,"Dy":0},
        "c":{"Name":"C","X":0,"Y":10,"Adj":["b"],"Dist":[3],"Line":"red","Shift":"left","Dx":0,"Dy":0},
        "d":{"Name":"D","X":10,"Y":10,"Adj":[],"Dist":[],"Line":"red","Shift":"left","Dx":0,"Dy":0}
    }"#).unwrap();
    let route = metro.route(StationId(0), StationId(1)).unwrap();
    assert_eq!(route.stations, [StationId(0), StationId(2), StationId(1)]);
    assert_eq!(route.minutes, 5);
    assert_eq!(route.transfers, 1);
    let stationary = metro.route(StationId(0), StationId(0)).unwrap();
    assert_eq!(stationary.stations, [StationId(0)]);
    assert_eq!(stationary.edges, [] as [usize; 0]);
    assert_eq!((stationary.minutes, stationary.transfers), (0, 0));
    assert!(metro.route(StationId(0), StationId(3)).is_none());
    assert!(metro.route(StationId(3), StationId(0)).is_none());
    assert!(metro.route(StationId(99), StationId(0)).is_none());
    assert!(metro.route(StationId(0), StationId(99)).is_none());
    assert!(metro.route(StationId(99), StationId(99)).is_none());
}
