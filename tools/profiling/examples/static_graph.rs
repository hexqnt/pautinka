use std::{env, error::Error, num::NonZeroUsize};

use egui::{Pos2, Rect, Vec2};
use pautinka_egui::{Canvas, CrossingMode, Graph, GridOptions, RoutingMode};
use pautinka_stress::scene::{Load, Scene, SceneStyle, Topology};
use pautinka_test_support::cpu::CpuUiHarness;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let frames = args
        .next()
        .map(|value| value.parse::<NonZeroUsize>())
        .transpose()?
        .map_or(120, NonZeroUsize::get);
    if args.next().is_some() {
        return Err("Usage: static_graph [frames=120]".into());
    }
    // Конечная нагрузка growth_static: те же стадии роста сохраняют исходное размещение вершин.
    let mut scene = Scene::with_options(
        42,
        Load::new(1_000, 3)?,
        SceneStyle {
            routing: RoutingMode::Straight,
            ..SceneStyle::default()
        },
        Topology::MIXED,
    );
    while let Some(load) = scene.load().next_stage() {
        scene.reconfigure(load, Topology::MIXED);
    }
    let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(1440.0, 1000.0));
    let mut csv = String::from(
        "scene,nodes,edges,view,action,minimap,visible_nodes,visible_edges,shapes,vertices,phase,samples,p50_ms,p95_ms,p99_ms,max_ms\n",
    );
    for minimap in [false, true] {
        for local in [false, true] {
            for action in ["idle", "pan", "zoom"] {
                let mut canvas = Canvas::default();
                canvas.options.navigator.enabled = minimap;
                canvas.options.grid = Some(GridOptions::default());
                canvas.style.label_min_zoom = 0.6;
                canvas.style.crossings.mode = CrossingMode::None;
                let mut harness = CpuUiHarness::default();
                let mut frame = |canvas: &mut Canvas| {
                    harness.frame(viewport, Vec::new(), |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            canvas.show(ui, &scene);
                        });
                    })
                };
                let mut warmup = Vec::with_capacity(10);
                for _ in 0..5 {
                    let sample = frame(&mut canvas);
                    warmup.push(sample.ui + sample.tessellation);
                }
                if local {
                    canvas.camera.zoom = 1.0;
                    canvas.camera.center = scene.node(scene.node_count() / 2).position;
                }
                for _ in 0..5 {
                    let sample = frame(&mut canvas);
                    warmup.push(sample.ui + sample.tessellation);
                }
                let samples: Vec<_> = (0..frames)
                    .map(|index| {
                        match action {
                            "pan" => {
                                let direction = if index % 2 == 0 { 1.0 } else { -1.0 };
                                canvas.camera.center +=
                                    Vec2::new(3.0, 1.0) * direction / canvas.camera.zoom;
                            }
                            "zoom" => canvas.camera.zoom_at(
                                viewport.center(),
                                if index % 2 == 0 { 1.02 } else { 1.0 / 1.02 },
                                viewport,
                            ),
                            _ => {}
                        }
                        frame(&mut canvas)
                    })
                    .collect();
                let last = samples.last().unwrap();
                let stats = canvas.stats();
                let view = if local { "local" } else { "fit" };
                let prefix = format!(
                    "static_graph,{},{},{view},{action},{minimap},{},{},{},{}",
                    scene.node_count(),
                    scene.edge_count(),
                    stats.node_candidates,
                    stats.edge_candidates,
                    last.shapes,
                    last.vertices,
                );
                pautinka_profiling::frame_rows(&mut csv, &prefix, "ui", &samples);
                pautinka_profiling::row(&mut csv, &prefix, "warmup_cpu_total", &mut warmup);
            }
        }
    }
    print!("{csv}");
    Ok(())
}
