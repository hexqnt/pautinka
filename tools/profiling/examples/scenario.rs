use std::{env, error::Error, fs, num::NonZeroUsize};

use pautinka_stress::{Demo, Scenario};
use pautinka_test_support::cpu::CpuUiHarness;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .ok_or("Usage: scenario <config.toml> [frames=120] [warmup=5]")?;
    if path == "--help" || path == "-h" {
        println!("Usage: scenario <config.toml> [frames=120] [warmup=5]");
        return Ok(());
    }
    let frames = args
        .next()
        .map(|value| value.parse::<NonZeroUsize>())
        .transpose()?
        .map_or(120, NonZeroUsize::get);
    let warmup = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(5);
    if let Some(argument) = args.next() {
        return Err(format!("Unexpected argument: {argument}").into());
    }

    let scenario: Scenario = fs::read_to_string(path)?.parse()?;
    let mut demo = Demo::with_scenario(scenario);
    // Фиксируем начальную нагрузку, сохраняя настройки движения и отрисовки сценария.
    demo.set_auto_grow(false);
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 1000.0));
    let mut harness = CpuUiHarness::default();
    let mut frame = || harness.frame(viewport, Vec::new(), |ui| demo.show(ui));
    for _ in 0..warmup {
        frame();
    }

    let samples: Vec<_> = (0..frames).map(|_| frame()).collect();
    let last = samples.last().unwrap();

    let scene = demo.scene();
    let prefix = format!(
        "scenario,{},{},{},{warmup},{},{}",
        scene.seed(),
        scene.load().nodes(),
        scene.load().edge_count(),
        last.shapes,
        last.vertices,
    );
    let mut csv = String::from(
        "scene,seed,nodes,edges,warmup,last_shapes,last_vertices,phase,samples,p50_ms,p95_ms,p99_ms,max_ms\n",
    );
    pautinka_profiling::frame_rows(&mut csv, &prefix, "ui", &samples);
    print!("{csv}");
    Ok(())
}
