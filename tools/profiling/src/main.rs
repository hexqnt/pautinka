mod allocations;
mod options;

use std::{error::Error, fmt::Write as _, fs, time::Instant};

use egui::Vec2;
use pautinka_profiling::{frame_rows, row};
use pautinka_test_support::canvas::{CpuCanvasHarness, Scene, SceneKind};

use allocations::{Scope, measured};
use options::{Action, Options, View};

fn canvas(csv: &mut String, memory: &mut String, allocations: &mut String, options: &Options) {
    for &size in &options.sizes {
        for kind in SceneKind::ALL
            .into_iter()
            .filter(|&kind| options.scene.is_none_or(|selected| selected == kind))
        {
            eprintln!("Profiling {kind}: {} nodes", size.get());
            let start = Instant::now();
            let scope = Scope {
                scene: kind,
                nodes: size.get(),
                view: "setup",
                action: "geometry",
                frame: 0,
            };
            let scene = measured(allocations, scope, || Scene::new(kind, size));
            let setup_prefix = format!(
                "canvas,{kind},{},{},setup,build,0,0,0,0",
                size.get(),
                size.get() * 3
            );
            row(csv, &setup_prefix, "geometry", &mut [start.elapsed()]);
            let mut harness = CpuCanvasHarness::new(scene);
            harness.canvas.options.lod.enabled = !options.no_lod;
            if options.bridges {
                harness.canvas.style.crossings.mode = pautinka_egui::CrossingMode::Bridge;
            }
            if options.smart_labels {
                harness.canvas.options.labels =
                    Some(pautinka_egui::labels::LabelLayoutOptions::default());
                harness.canvas.style.label_min_zoom = 0.0;
            }
            row(
                csv,
                &setup_prefix,
                "prepare",
                &mut [measured(
                    allocations,
                    Scope {
                        action: "prepare",
                        ..scope
                    },
                    || harness.prepare(),
                )],
            );
            let first = measured(
                allocations,
                Scope {
                    action: "first_frame",
                    ..scope
                },
                || harness.frame(Vec::new()),
            );
            let prefix = format!(
                "canvas,{kind},{},{},setup,first_frame,0,0,{},{}",
                size.get(),
                size.get() * 3,
                first.shapes,
                first.vertices
            );
            row(
                csv,
                &prefix,
                "cpu_total",
                &mut [first.ui + first.tessellation],
            );
            for view in View::ALL
                .into_iter()
                .filter(|&view| options.view.is_none_or(|selected| selected == view))
            {
                for action in Action::ALL
                    .into_iter()
                    .filter(|&action| options.action.is_none_or(|selected| selected == action))
                {
                    harness.canvas.options.interaction.snapping = action == Action::SnapMove;
                    harness.canvas.options.snapping.kinds = pautinka_egui::SnapKinds::ALL;
                    harness.frame(vec![egui::Event::PointerGone]);
                    harness.canvas.camera.fit = true;
                    harness.frame(Vec::new());
                    if view == View::Local {
                        harness.canvas.camera.zoom = 1.0;
                        harness.canvas.camera.center = harness.scene.positions[size.get() / 2];
                    }
                    harness.frame(Vec::new());
                    for _ in 0..5 {
                        harness.frame(Vec::new());
                    }
                    let mut samples = Vec::with_capacity(options.frames.get());
                    let moving_node = size.get() / 2;
                    let drag_origin = harness
                        .canvas
                        .camera
                        .to_screen(harness.scene.positions[moving_node], harness.viewport);
                    let before_updates = harness.canvas.stats().updated_entries;
                    let before_alignment = harness.canvas.stats().snap_rebuilds;
                    for frame in 0..options.frames.get() {
                        let mut events = Vec::new();
                        match action {
                            Action::Pan => {
                                // Длинный профиль остаётся рядом с исходной областью вместо ухода за границы графа.
                                let direction = if (frame / 120).is_multiple_of(2) {
                                    1.0
                                } else {
                                    -1.0
                                };
                                harness.canvas.camera.center +=
                                    Vec2::new(3.0, 1.0) * direction / harness.canvas.camera.zoom;
                            }
                            Action::Zoom => harness.canvas.camera.zoom_at(
                                harness.viewport.center(),
                                if frame % 2 == 0 { 1.02 } else { 1.0 / 1.02 },
                                harness.viewport,
                            ),
                            Action::Pick => {
                                let point = harness.viewport.center()
                                    + Vec2::new((frame % 2) as f32 * 80.0, 40.0);
                                events.push(egui::Event::PointerMoved(point));
                                for pressed in [true, false] {
                                    events.push(egui::Event::PointerButton {
                                        pos: point,
                                        button: egui::PointerButton::Primary,
                                        pressed,
                                        modifiers: egui::Modifiers::NONE,
                                    });
                                }
                            }
                            Action::Move | Action::SnapMove => {
                                if frame == 0 {
                                    events.push(egui::Event::PointerMoved(drag_origin));
                                    events.push(egui::Event::PointerButton {
                                        pos: drag_origin,
                                        button: egui::PointerButton::Primary,
                                        pressed: true,
                                        modifiers: egui::Modifiers::NONE,
                                    });
                                } else {
                                    events.push(egui::Event::PointerMoved(
                                        drag_origin
                                            + Vec2::new(
                                                (frame as f32 * 0.1).sin() * 40.0,
                                                (frame as f32 * 0.1).cos() * 20.0,
                                            ),
                                    ));
                                }
                            }
                            Action::Idle => {}
                        }
                        let sample = measured(
                            allocations,
                            Scope {
                                view: view.as_str(),
                                action: action.as_str(),
                                frame,
                                ..scope
                            },
                            || harness.frame(events),
                        );
                        samples.push(sample);
                    }
                    if matches!(action, Action::Move | Action::SnapMove) {
                        assert!(
                            harness.canvas.stats().updated_entries > before_updates,
                            "Moving scenario must update graph geometry"
                        );
                        harness.frame(vec![egui::Event::PointerButton {
                            pos: drag_origin,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::NONE,
                        }]);
                    }
                    if action == Action::SnapMove {
                        let rebuilds = harness.canvas.stats().snap_rebuilds;
                        assert!(
                            rebuilds > 0 && rebuilds - before_alignment <= 1,
                            "Snapping gesture must build its source index at most once"
                        );
                    }
                    let (visible_nodes, visible_edges) = harness
                        .scene
                        .visible_counts(&harness.canvas, harness.viewport);
                    let view = view.as_str();
                    let action = action.as_str();
                    let last = samples.last().unwrap();
                    let prefix = format!(
                        "canvas,{kind},{},{},{view},{action},{visible_nodes},{visible_edges},{},{}",
                        size.get(),
                        size.get() * 3,
                        last.shapes,
                        last.vertices
                    );
                    frame_rows(csv, &prefix, "ui", &samples);
                    let stats = harness.canvas.stats();
                    writeln!(
                        memory,
                        "{kind},{},{},{view},{action},{},{},{},{},{},{},{},{}",
                        size.get(),
                        size.get() * 3,
                        stats.spatial_entries,
                        stats.spatial_bytes,
                        stats.spatial_cache_bytes,
                        stats.node_candidates,
                        stats.edge_candidates,
                        stats.rebuilds,
                        stats.updated_entries,
                        stats.snap_rebuilds
                    )
                    .unwrap();
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let mut csv = String::from(
        "target,scene,nodes,edges,view,action,visible_nodes,visible_edge_bounds,shapes,vertices,phase,samples,p50_ms,p95_ms,p99_ms,max_ms\n",
    );
    let mut memory = String::from(
        "scene,nodes,edges,view,action,index_entries,index_bytes,spatial_cache_bytes,node_candidates,edge_candidates,rebuilds,updated_entries,snap_rebuilds\n",
    );
    let mut allocations = String::from(
        "scene,nodes,edges,view,action,frame,allocations,allocated_bytes,net_bytes,peak_growth_bytes\n",
    );
    canvas(&mut csv, &mut memory, &mut allocations, &options);
    if let Some(path) = options.output {
        fs::write(path.with_extension("memory.csv"), memory)?;
        #[cfg(feature = "count-allocations")]
        fs::write(path.with_extension("allocations.csv"), allocations)?;
        fs::write(path, csv)?;
    } else {
        print!("{csv}");
    }
    Ok(())
}
