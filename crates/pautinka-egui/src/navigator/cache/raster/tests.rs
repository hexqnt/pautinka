#![expect(
    clippy::cast_sign_loss,
    reason = "Точки и размеры тестовой мини-карты положительны."
)]

use egui::{Vec2, pos2};

use crate::{
    Edge, EdgeId, EdgePath, GraphId, GraphRevision, Group, GroupFrame, GroupId, NavigatorCorner,
    Node, NodeId,
};

use super::*;

struct Scene {
    id: GraphId,
    node: bool,
    path: Option<EdgePath>,
    group: bool,
}

impl Scene {
    fn new() -> Self {
        Self {
            id: GraphId::fresh(),
            node: false,
            path: None,
            group: false,
        }
    }
}

impl Graph for Scene {
    fn id(&self) -> GraphId {
        self.id
    }
    fn revision(&self) -> GraphRevision {
        GraphRevision::default()
    }
    fn bounds(&self) -> Rect {
        Rect::from_min_max(pos2(-20.0, -20.0), pos2(20.0, 20.0))
    }
    fn node_count(&self) -> usize {
        usize::from(self.node)
    }
    fn edge_count(&self) -> usize {
        usize::from(self.path.is_some())
    }
    fn group_count(&self) -> usize {
        usize::from(self.group)
    }
    fn node(&self, _: usize) -> Node<'_> {
        Node {
            color: Color32::RED,
            ..Node::new(NodeId::new(0), Pos2::ZERO, "")
        }
    }
    fn edge(&self, _: usize) -> Edge<'_> {
        Edge {
            color: Color32::BLUE,
            is_loop: true,
            ..Edge::new(EdgeId::new(0), self.path.as_ref().unwrap())
        }
    }
    fn group(&self, _: usize) -> Group<'_> {
        let mut group = Group::new(
            GroupId::new(0),
            GroupFrame::new(self.bounds(), 2.0).unwrap(),
            "",
        );
        group.color = Color32::GREEN;
        group
    }
}

fn navigator(graph: &Scene) -> Navigator {
    Navigator::new(
        Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
        graph.bounds(),
        NavigatorCorner::TopRight,
    )
    .unwrap()
}

fn pixel(image: &ColorImage, navigator: &Navigator, world: Pos2, scale: f32) -> Color32 {
    let point = navigator.camera.to_screen(
        world,
        Rect::from_min_size(Pos2::ZERO, navigator.thumbnail.size()),
    ) * scale;
    image.pixels[point.y as usize * image.size[0] + point.x as usize]
}

#[test]
fn raster_preserves_scale_transparency_groups_and_paint_order() {
    let mut scene = Scene::new();
    scene.node = true;
    scene.group = true;
    scene.path = Some(EdgePath::try_polyline(vec![pos2(-20.0, 0.0), pos2(20.0, 0.0)]).unwrap());
    let navigator = navigator(&scene);
    for scale in [1.0, 1.5, 2.0] {
        let image = image(&scene, &navigator, true, scale, true).unwrap();
        assert_eq!(
            image.size,
            [
                (navigator.thumbnail.width() * scale).ceil() as usize,
                (navigator.thumbnail.height() * scale).ceil() as usize
            ]
        );
        assert_eq!(image.pixels[0], Color32::TRANSPARENT);
        assert_eq!(pixel(&image, &navigator, Pos2::ZERO, scale), Color32::RED);
        assert!(image.pixels.contains(&Color32::GREEN));
        assert!(
            image
                .pixels
                .iter()
                .any(|color| color.b() > 0 && color.r() == 0 && color.g() == 0)
        );
    }
}

#[test]
fn raster_keeps_bends_and_curves_and_hides_loops() {
    for path in [
        EdgePath::try_polyline(vec![
            pos2(-10.0, 0.0),
            pos2(-10.0, 10.0),
            pos2(10.0, 10.0),
            pos2(10.0, 0.0),
        ])
        .unwrap(),
        EdgePath::try_quadratic([pos2(-10.0, 0.0), pos2(0.0, 20.0), pos2(10.0, 0.0)]).unwrap(),
        EdgePath::try_cubic([
            pos2(-10.0, 0.0),
            pos2(-10.0, 20.0),
            pos2(10.0, 20.0),
            pos2(10.0, 0.0),
        ])
        .unwrap(),
    ] {
        let midpoint = path.midpoint();
        let mut scene = Scene::new();
        scene.path = Some(path);
        let navigator = navigator(&scene);
        let drawn = image(&scene, &navigator, true, 1.0, true).unwrap();
        assert!(pixel(&drawn, &navigator, midpoint, 1.0).b() > 0);
        assert_eq!(
            pixel(&drawn, &navigator, Pos2::ZERO, 1.0),
            Color32::TRANSPARENT
        );
        let hidden = image(&scene, &navigator, false, 1.0, true).unwrap();
        assert!(
            hidden
                .pixels
                .iter()
                .all(|color| *color == Color32::TRANSPARENT)
        );
    }
}

#[test]
fn raster_ignores_corner_rounding() {
    let mut scene = Scene::new();
    scene.path = Some(
        EdgePath::try_polyline([pos2(-10.0, -10.0), pos2(10.0, -10.0), pos2(10.0, 10.0)]).unwrap(),
    );
    let navigator = navigator(&scene);
    let sharp = image(&scene, &navigator, true, 2.0, true).unwrap();
    scene.path = Some(
        scene
            .path
            .take()
            .unwrap()
            .with_corner_radius(crate::CornerRadius::try_from(8.0).unwrap())
            .unwrap(),
    );
    assert!(scene.path.as_ref().unwrap().points().len() > 3);
    let rounded = image(&scene, &navigator, true, 2.0, true).unwrap();
    assert_eq!(rounded.size, sharp.size);
    assert_eq!(rounded.pixels, sharp.pixels);
}
