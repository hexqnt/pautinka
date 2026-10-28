use std::{
    env,
    error::Error,
    iter::Peekable,
    str::FromStr,
    time::{Duration, Instant},
};

use pautinka_egui::{Canvas, CrossingMode, Graph};
use pautinka_stress::scene::{Load, Motion, Scene, SceneStyle, Topology};
use pautinka_test_support::cpu::CpuUiHarness;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1).peekable();
    if args
        .peek()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        println!(
            "Usage: stress [nodes=1000] [frames=120] [--ratio 3.0] [--locality 85] [--radius 3] [--hub-share 0] [--hubs 4] [--no-backbone] [--seed 42] [--model-only] [--minimap]"
        );
        return Ok(());
    }
    let nodes = positional(&mut args, 1_000_usize)?;
    let frames = positional(&mut args, 120_usize)?;
    if frames == 0 {
        return Err("Frame count must be positive".into());
    }
    let mut ratio = 3.0;
    let mut seed = 42;
    let defaults = Topology::default();
    let mut locality = defaults.locality();
    let mut radius = defaults.local_radius();
    let mut hub_share = defaults.hub_share();
    let mut hubs = defaults.hubs();
    let mut backbone = defaults.backbone();
    let mut model_only = false;
    let mut minimap = false;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--ratio" => ratio = value(&mut args)?,
            "--seed" => seed = value(&mut args)?,
            "--locality" => locality = value(&mut args)?,
            "--radius" => radius = value(&mut args)?,
            "--hub-share" => hub_share = value(&mut args)?,
            "--hubs" => hubs = value(&mut args)?,
            "--no-backbone" => backbone = false,
            "--model-only" => model_only = true,
            "--minimap" => minimap = true,
            _ => return Err(format!("Unknown option: {flag}").into()),
        }
    }
    let load = Load::new(nodes, ratio)?;
    let topology = Topology::new(locality, radius, hub_share, hubs, backbone)?;
    let mut scene = Scene::with_options(seed, load, SceneStyle::default(), topology);
    let mut canvas = Canvas::default();
    canvas.set_labels(false);
    canvas.options.lod.enabled = false;
    canvas.options.navigator.enabled = minimap;
    canvas.style.crossings.mode = CrossingMode::None;
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 1000.0));
    let mut harness = CpuUiHarness::default();
    let mut model_times = Vec::with_capacity(frames);
    let mut samples = Vec::with_capacity(frames);
    let step = Duration::from_secs_f64(1.0 / 60.0);
    let mut frame = || {
        // Один шаг модели на кадр, независимо от числа проходов egui.
        let start = Instant::now();
        scene.advance(step, Motion::default());
        let model = start.elapsed();
        let sample = (!model_only).then(|| {
            harness.frame(viewport, Vec::new(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    canvas.show(ui, &scene);
                });
            })
        });
        (model, sample)
    };
    for _ in 0..5 {
        frame();
    }
    for _ in 0..frames {
        let (model, sample) = frame();
        model_times.push(model);
        if let Some(mut sample) = sample {
            sample.ui = model + sample.ui;
            samples.push(sample);
        }
    }
    let (shapes, vertices) = samples
        .last()
        .map_or((0, 0), |sample| (sample.shapes, sample.vertices));
    let mut csv = String::from(
        "scene,seed,nodes,edges,ratio,locality_pct,local_radius,hub_share_pct,hubs,backbone,minimap,last_shapes,last_vertices,phase,samples,p50_ms,p95_ms,p99_ms,max_ms\n",
    );
    let prefix = format!(
        "stress,{seed},{nodes},{},{:.2},{locality},{radius},{hub_share},{hubs},{backbone},{minimap},{shapes},{vertices}",
        scene.edge_count(),
        load.edges_per_node()
    );
    pautinka_profiling::row(&mut csv, &prefix, "model", &mut model_times);
    if !samples.is_empty() {
        pautinka_profiling::frame_rows(&mut csv, &prefix, "model_and_ui", &samples);
    }
    print!("{csv}");
    Ok(())
}

fn value<T: FromStr>(args: &mut impl Iterator<Item = String>) -> Result<T, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    Ok(args.next().ok_or("Expected an option value")?.parse()?)
}

fn positional<T: FromStr>(
    args: &mut Peekable<impl Iterator<Item = String>>,
    default: T,
) -> Result<T, Box<dyn Error>>
where
    T::Err: Error + 'static,
{
    if args.peek().is_some_and(|arg| !arg.starts_with('-')) {
        value(args)
    } else {
        Ok(default)
    }
}
