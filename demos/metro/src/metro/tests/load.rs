use egui::pos2;

use super::*;

#[test]
fn invalid_topology_is_rejected() {
    for data in [
        "{}".to_owned(),
        City::SaintPetersburg.data().replacen("st_g1", "missing", 1),
        City::SaintPetersburg
            .data()
            .replacen("\"Dist\": [4]", "\"Dist\": []", 1),
        City::SaintPetersburg
            .data()
            .replacen("\"Dist\": [4]", "\"Dist\": [0]", 1),
        City::SaintPetersburg
            .data()
            .replacen("\"green\"", "\"unknown\"", 1),
        City::SaintPetersburg
            .data()
            .replacen("\"transit_only\"", "\"unknown\"", 1),
    ] {
        assert!(Metro::parse(&data).is_err());
    }
}

#[test]
fn reciprocal_and_duplicate_connections_share_an_edge_and_reject_conflicting_weights() {
    let mut data = serde_json::json!({
        "b": {
            "Name": "Станция \"Б\"", "X": 10, "Y": 0,
            "Adj": ["a"], "Dist": [2],
            "Line": "red", "Shift": "left", "Dx": 0, "Dy": 0
        },
        "a": {
            "Name": "Станция А", "X": 0, "Y": 0,
            "Adj": ["b", "b"], "Dist": [2, 2],
            "Line": "red", "Shift": "left", "Dx": 0, "Dy": 0
        }
    });
    let metro = Metro::parse(&data.to_string()).unwrap();
    assert_eq!(metro.stations[0].name, "Станция А");
    assert_eq!(metro.stations[1].name, "Станция \"Б\"");
    assert_eq!(metro.connections.len(), 1);
    let edge = &metro.connections[0];
    assert_eq!(
        (edge.from, edge.to, edge.minutes),
        (StationId(0), StationId(1), 2)
    );
    assert_eq!(
        metro.adjacency,
        [vec![(StationId(1), 0)], vec![(StationId(0), 0)]]
    );
    assert_eq!(edge.path.points(), [pos2(0.0, 0.0), pos2(20.0, 0.0)]);

    for minutes in [1, 3] {
        data["b"]["Dist"][0] = minutes.into();
        assert_eq!(
            Metro::parse(&data.to_string()).err().unwrap(),
            "Conflicting connection: Станция \"Б\" -> a"
        );
    }
    data["b"]["Dist"][0] = 2.into();
    for minutes in [1, 3] {
        data["a"]["Dist"][1] = minutes.into();
        assert_eq!(
            Metro::parse(&data.to_string()).err().unwrap(),
            "Conflicting connection: Станция А -> b"
        );
    }
    data["a"]["Adj"][0] = "a".into();
    assert_eq!(
        Metro::parse(&data.to_string()).err().unwrap(),
        "Self connection: a"
    );
}

#[test]
fn connections_keep_the_first_orientation_and_input_order_with_reciprocal_links() {
    let metro = Metro::parse(r#"{
        "c":{"Name":"C","X":0,"Y":10,"Adj":["a","b","b"],"Dist":[2,3,3],"Line":"red","Shift":"left","Dx":0,"Dy":0},
        "a":{"Name":"A","X":0,"Y":0,"Adj":["c","b"],"Dist":[2,1],"Line":"red","Shift":"left","Dx":0,"Dy":0},
        "b":{"Name":"B","X":10,"Y":0,"Adj":["a"],"Dist":[1],"Line":"red","Shift":"left","Dx":0,"Dy":0}
    }"#).unwrap();
    assert_eq!(
        metro
            .stations
            .iter()
            .map(|station| station.name.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", "C"]
    );
    assert_eq!(
        metro
            .connections
            .iter()
            .map(|edge| (edge.from.0, edge.to.0, edge.minutes))
            .collect::<Vec<_>>(),
        [(0, 2, 2), (0, 1, 1), (2, 1, 3)]
    );
    let route = metro.route(StationId(1), StationId(2)).unwrap();
    assert_eq!(route.minutes, 3);
    assert_eq!(route.stations, [StationId(1), StationId(2)]);
}
