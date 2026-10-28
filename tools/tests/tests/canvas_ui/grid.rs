use super::*;
use pautinka_egui::{Camera, DefaultRenderer, GridKind, GridOptions, RenderContext, Renderer};

fn grid_color(color: Color32) -> bool {
    color.r() > 0 && color.g() == 0 && color.b() == 0
}

fn options() -> GridOptions {
    let mut grid = GridOptions::new(10.0, 10.0).unwrap();
    grid.minor_color = Color32::RED;
    grid.major_color = Color32::RED;
    grid
}

#[test]
fn grid_follows_pan_and_pointer_zoom_and_stays_below_the_graph() {
    let mut state = State::default();
    state.canvas.options.grid = Some(options());
    state.canvas.options.camera.limit_zoom_out_to_fit = false;
    state.canvas.set_camera(Camera {
        center: Pos2::ZERO,
        zoom: 2.0,
        fit: false,
    });
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .with_max_steps(120)
        .build_ui_state(|ui, state| State::show(state, ui), state);
    harness.run();
    let check_grid = |harness: &Harness<'_, State>| {
        let viewport = harness.get_by_label("Graph canvas").rect();
        let origin = harness
            .state()
            .canvas
            .camera
            .to_screen(Pos2::ZERO, viewport);
        let shapes = &harness.output().shapes;
        let background = shapes.iter().position(|shape| matches!(&shape.shape,
            egui::Shape::Rect(rect) if rect.fill == harness.state().canvas.style.background && rect.rect == viewport
        )).unwrap();
        let node = shapes
            .iter()
            .position(|shape| {
                matches!(&shape.shape,
                    egui::Shape::Circle(circle) if circle.fill == harness.state().graph.node_color
                )
            })
            .unwrap();
        let mut vertical_origin = false;
        let mut horizontal_origin = false;
        let mut count = 0;
        for (index, shape) in shapes.iter().enumerate() {
            if let egui::Shape::LineSegment { points, stroke } = &shape.shape
                && grid_color(stroke.color)
            {
                count += 1;
                assert!(index > background && index < node);
                assert!(viewport.contains_rect(shape.clip_rect));
                assert!(points.iter().all(|&point| viewport.contains(point)));
                vertical_origin |=
                    (points[0].x - origin.x).abs() < 0.001 && points[0].x == points[1].x;
                horizontal_origin |=
                    (points[0].y - origin.y).abs() < 0.001 && points[0].y == points[1].y;
            }
        }
        assert!(count > 10 && count <= 1026);
        assert!(vertical_origin && horizontal_origin);
    };
    check_grid(&harness);
    let viewport = harness.get_by_label("Graph canvas").rect();
    let from = viewport.center() + Vec2::new(0.0, 120.0);
    let before = harness.state().canvas.camera;
    pointer_gesture(
        &mut harness,
        from,
        from + Vec2::new(31.0, 17.0),
        egui::PointerButton::Primary,
        egui::Modifiers::NONE,
    );
    assert_ne!(harness.state().canvas.camera.center, before.center);
    check_grid(&harness);
    let pointer = from + Vec2::new(40.0, 20.0);
    let before = harness.state().canvas.camera;
    let anchor = before.to_world(pointer, viewport);
    harness.hover_at(pointer);
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -300.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
    assert!(harness.state().canvas.camera.zoom < before.zoom);
    assert!(
        harness
            .state()
            .canvas
            .camera
            .to_screen(anchor, viewport)
            .distance(pointer)
            < 0.001
    );
    check_grid(&harness);
    click_label(&mut harness, "First");
    assert_eq!(harness.state().canvas.selected(), Some(Selection::Node(0)));
    harness.state_mut().canvas.options.grid = None;
    harness.run();
    assert!(
        !harness
            .output()
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape,
                egui::Shape::LineSegment { stroke, .. } if grid_color(stroke.color)
            ))
    );
}

#[test]
fn dots_use_world_intersections_and_respect_the_viewport() {
    let mut state = State::default();
    let mut grid = options();
    grid.kind = GridKind::Dots;
    state.canvas.options.grid = Some(grid);
    state.canvas.set_camera(Camera {
        center: pos2(-27.0, 13.0),
        zoom: 2.0,
        fit: false,
    });
    let mut harness = canvas_harness(state);
    harness.run();
    let viewport = harness.get_by_label("Graph canvas").rect();
    let camera = harness.state().canvas.camera;
    let mut count = 0;
    for shape in &harness.output().shapes {
        if let egui::Shape::Circle(circle) = &shape.shape
            && grid_color(circle.fill)
        {
            count += 1;
            assert!(viewport.contains_rect(shape.clip_rect));
            assert!(viewport.contains(circle.center));
            let world = camera.to_world(circle.center, viewport) / 10.0;
            assert!((world.x - world.x.round()).abs() < 0.001);
            assert!((world.y - world.y.round()).abs() < 0.001);
        }
    }
    assert!(count > 100 && count < 17_000);
}

#[test]
fn partial_clipping_preserves_grid_spacing_colors_and_sizes() {
    let mut state = State::default();
    let mut grid = options().with_spacing(10.0, 8.0).unwrap();
    grid.kind = GridKind::Dots;
    state.canvas.options.grid = Some(grid);
    state.canvas.set_camera(Camera {
        center: pos2(-27.0, 13.0),
        zoom: 1.0,
        fit: false,
    });
    let mut harness = Harness::builder()
        .with_size(Vec2::new(2000.0, 1500.0))
        .build_ui_state(
            |ui, (state, clip): &mut (State, Option<Rect>)| {
                if let Some(clip) = clip {
                    ui.set_clip_rect(*clip);
                }
                state.canvas.show(ui, &state.graph);
            },
            (state, None),
        );
    harness.run();
    let clip = harness
        .get_by_label("Graph canvas")
        .rect()
        .shrink2(Vec2::new(800.0, 550.0));
    let dots = |harness: &Harness<'_, (State, Option<Rect>)>| {
        harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Circle(circle)
                    if grid_color(circle.fill) && clip.contains(circle.center) =>
                {
                    Some((circle.center, circle.radius, circle.fill))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let full = dots(&harness);
    assert_ne!(full, [] as [(Pos2, f32, Color32); 0]);
    harness.state_mut().1 = Some(clip);
    harness.run();
    assert_eq!(dots(&harness), full);
}

#[test]
fn custom_background_can_extend_or_replace_the_grid_even_on_an_empty_graph() {
    struct EmptyGraph(pautinka_egui::GraphId);
    impl Graph for EmptyGraph {
        fn id(&self) -> pautinka_egui::GraphId {
            self.0
        }
        fn revision(&self) -> pautinka_egui::GraphRevision {
            pautinka_egui::GraphRevision::default()
        }
        fn node_count(&self) -> usize {
            0
        }
        fn edge_count(&self) -> usize {
            0
        }
        fn node(&self, _: usize) -> Node<'_> {
            unreachable!("Empty graph has no nodes")
        }
        fn edge(&self, _: usize) -> Edge<'_> {
            unreachable!("Empty graph has no edges")
        }
        fn bounds(&self) -> Rect {
            Rect::from_center_size(Pos2::ZERO, Vec2::splat(100.0))
        }
    }
    #[derive(Default)]
    struct Background {
        camera: std::cell::Cell<Option<Camera>>,
        replace_grid: bool,
    }
    impl Renderer for Background {
        fn paint_background(&self, context: &RenderContext<'_>, grid: Option<&GridOptions>) {
            self.camera.set(Some(context.camera));
            if !self.replace_grid {
                DefaultRenderer.paint_background(context, grid);
            }
            context
                .painter
                .rect_filled(context.viewport.shrink(15.0), 0.0, Color32::MAGENTA);
        }
    }
    let mut canvas = Canvas::default();
    canvas.options.grid = Some(options());
    let mut harness = Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui_state(
            |ui, (canvas, graph, background): &mut (Canvas, EmptyGraph, Background)| {
                canvas.show_with(ui, graph, background);
            },
            (
                canvas,
                EmptyGraph(pautinka_egui::GraphId::fresh()),
                Background::default(),
            ),
        );
    harness.run();
    for replace in [false, true] {
        harness.state_mut().2.replace_grid = replace;
        harness.run();
        assert_eq!(
            harness.state().2.camera.get(),
            Some(harness.state().0.camera)
        );
        assert!(
            harness
                .output()
                .shapes
                .iter()
                .any(|shape| matches!(&shape.shape,
                    egui::Shape::Rect(rect) if rect.fill == Color32::MAGENTA
                ))
        );
        let has_grid = harness.output().shapes.iter().any(|shape| {
            matches!(&shape.shape,
                egui::Shape::LineSegment { stroke, .. } if grid_color(stroke.color)
            )
        });
        assert_eq!(has_grid, !replace);
    }
}
