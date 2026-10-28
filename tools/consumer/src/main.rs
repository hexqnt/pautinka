#[cfg(feature = "egui")]
mod ui;

use pautinka::{NodeShape, RoutingMode, SpatialIndex, Vec2, pos2};

fn main() {
    let shape = NodeShape::rectangle(Vec2::new(40.0, 20.0)).unwrap();
    let outlet = shape.anchor(pos2(-100.0, 0.0), Vec2::X).unwrap();
    let inlet = shape.anchor(pos2(100.0, 0.0), -Vec2::X).unwrap();
    let path = RoutingMode::Curved
        .route_anchors(outlet, inlet, 40.0, 0.0)
        .unwrap();
    let mut index = SpatialIndex::default();
    let handle = index.insert(7_u64, path.bounds()).unwrap();
    let mut found = false;
    index.query(path.bounds(), |id| found |= id == 7);
    assert!(found);
    assert_eq!(path.points()[0], outlet.position());
    assert!(path.length() > 0.0);
    index.remove(handle).unwrap();
    assert!(index.is_empty());
    println!("Independent Pautinka geometry consumer passed");

    #[cfg(feature = "egui")]
    ui::run(shape, path);
}
