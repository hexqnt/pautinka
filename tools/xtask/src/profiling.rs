use std::{
    collections::BTreeMap,
    fs,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    io::{hash_file, write_json},
    output, run,
};

#[derive(Args)]
pub struct Options {
    #[arg(long)]
    baseline: PathBuf,
    #[arg(long)]
    current: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "120")]
    frames: NonZeroUsize,
    #[arg(long, default_value = "3")]
    runs: NonZeroUsize,
    #[arg(long, default_value = "10000")]
    nodes: NonZeroUsize,
}

#[derive(Deserialize)]
struct Row {
    target: String,
    scene: String,
    nodes: usize,
    edges: usize,
    view: String,
    action: String,
    phase: String,
    samples: usize,
    #[serde(flatten)]
    sample: Sample,
}

#[derive(Deserialize)]
struct Sample {
    p50_ms: f64,
    p95_ms: f64,
    shapes: u64,
    vertices: u64,
}

#[derive(Clone, Copy)]
struct Workload<'a> {
    scene: &'a str,
    view: &'a str,
    action: &'a str,
    nodes: usize,
    edges: usize,
    frames: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    CpuTotal,
    Tessellation,
    Ui,
}

impl Row {
    fn read(reader: impl std::io::Read, expected: Workload<'_>) -> Result<BTreeMap<Phase, Sample>> {
        let mut rows = BTreeMap::new();
        for row in csv::Reader::from_reader(reader).deserialize::<Self>() {
            let row = row?;
            if row.view != expected.view || row.action != expected.action {
                continue;
            }
            ensure!(
                row.target == "canvas"
                    && row.scene == expected.scene
                    && row.nodes == expected.nodes
                    && row.edges == expected.edges,
                "Unexpected workload: {}/{}, {} nodes, {} edges in {}",
                row.target,
                row.scene,
                row.nodes,
                row.edges,
                row.phase
            );
            ensure!(
                row.samples == expected.frames,
                "Expected {} samples, got {} in {}",
                expected.frames,
                row.samples,
                row.phase
            );
            ensure!(
                row.sample.p50_ms.is_finite()
                    && row.sample.p95_ms.is_finite()
                    && row.sample.p50_ms >= 0.0
                    && row.sample.p95_ms >= row.sample.p50_ms,
                "Invalid profile timing in {}",
                row.phase
            );
            let phase = match row.phase.as_str() {
                "cpu_total" => Phase::CpuTotal,
                "tessellation" => Phase::Tessellation,
                "ui" => Phase::Ui,
                _ => anyhow::bail!("Unexpected frame phase: {}", row.phase),
            };
            ensure!(
                rows.insert(phase, row.sample).is_none(),
                "Duplicate frame phase: {}",
                row.phase
            );
        }
        ensure!(
            rows.len() == 3,
            "Expected ui, tessellation and cpu_total phases for {}/{}/{}",
            expected.scene,
            expected.view,
            expected.action
        );
        Ok(rows)
    }
}

#[derive(Serialize)]
struct Summary<'a> {
    scene: &'a str,
    view: &'a str,
    action: &'a str,
    variant: &'a str,
    phase: Phase,
    runs: usize,
    median_p50_ms: f64,
    min_p50_ms: f64,
    max_p50_ms: f64,
    median_p95_ms: f64,
    shapes: u64,
    vertices: u64,
}

fn median(values: &mut [f64]) -> Result<f64> {
    values.sort_unstable_by(f64::total_cmp);
    let middle = values.len() / 2;
    let &upper = values.get(middle).context("Empty profile series")?;
    Ok(if values.len().is_multiple_of(2) {
        f64::midpoint(values[middle - 1], upper)
    } else {
        upper
    })
}

fn cpu_from_affinity(affinity: &str) -> Result<usize> {
    let ranges = affinity
        .rsplit_once(':')
        .context("Unexpected taskset output")?
        .1
        .trim();
    let mut first = None;
    for range in ranges.split(',') {
        let (start, end) = range.split_once('-').unwrap_or((range, range));
        let start: usize = start.parse()?;
        let end: usize = end.parse()?;
        ensure!(start <= end, "Invalid CPU range: {range}");
        first.get_or_insert(start);
        if (start..=end).contains(&2) {
            return Ok(2);
        }
    }
    first.context("No available CPUs")
}

fn command_text(command: &mut Command) -> Result<String> {
    Ok(String::from_utf8(output(command)?.stdout)?
        .trim()
        .to_owned())
}

fn prepare_output(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    ensure!(
        fs::read_dir(path)?.next().is_none(),
        "Output directory must be empty: {}",
        path.display()
    );
    Ok(())
}

pub fn compare(options: &Options) -> Result<()> {
    ensure!(
        cfg!(target_os = "linux"),
        "CPU comparison requires Linux and taskset"
    );
    ensure!(
        options.frames.get() >= 3,
        "Comparison requires at least three frames for drag events"
    );
    let edges = options
        .nodes
        .get()
        .checked_mul(3)
        .context("Node count overflow")?;
    let affinity = command_text(
        Command::new("taskset")
            .env("LC_ALL", "C")
            .args(["-pc", &std::process::id().to_string()]),
    )?;
    let cpu = cpu_from_affinity(&affinity)?;
    let baseline = options.baseline.canonicalize().with_context(|| {
        format!(
            "Failed to locate baseline binary {}",
            options.baseline.display()
        )
    })?;
    let current = options.current.canonicalize().with_context(|| {
        format!(
            "Failed to locate current binary {}",
            options.current.display()
        )
    })?;
    prepare_output(&options.output)?;
    let binaries = [
        ("baseline", &baseline),
        ("full", &current),
        ("lod", &current),
    ];
    let cases = [
        ("network", "fit", "idle"),
        ("orthogonal", "fit", "idle"),
        ("network", "local", "pan"),
        ("network", "local", "pick"),
        ("orthogonal", "local", "move"),
    ];
    let mut records = Records::new();
    for index in 0..options.runs.get() {
        for (scene, view, action) in cases {
            let mut variants = binaries;
            if index % 2 == 1 {
                variants.reverse();
            }
            for (variant, binary) in variants
                .into_iter()
                .filter(|(variant, _)| view == "fit" || *variant != "lod")
            {
                let filename = options.output.join(format!(
                    "{scene}-{view}-{action}-{variant}-{}.csv",
                    index + 1
                ));
                let mut command = Command::new("taskset");
                command
                    .args(["-c", &cpu.to_string()])
                    .arg(binary)
                    .args([
                        "--sizes",
                        &options.nodes.to_string(),
                        "--scene",
                        scene,
                        "--view",
                        view,
                        "--action",
                        action,
                        "--canvas-only",
                        "--frames",
                        &options.frames.to_string(),
                        "--output",
                    ])
                    .arg(&filename);
                if variant != "lod" {
                    command.arg("--no-lod");
                }
                println!("Run {}: {scene}/{view}/{action}/{variant}", index + 1);
                run(&mut command)?;
                let workload = Workload {
                    scene,
                    view,
                    action,
                    nodes: options.nodes.get(),
                    edges,
                    frames: options.frames.get(),
                };
                for (phase, sample) in Row::read(fs::File::open(&filename)?, workload)
                    .with_context(|| format!("Invalid profile {}", filename.display()))?
                {
                    records
                        .entry((scene, view, action, variant, phase))
                        .or_default()
                        .push(sample);
                }
            }
        }
    }
    write_summary(&options.output.join("summary.csv"), &records)?;
    write_metadata(options, cpu, edges, &binaries)?;
    println!("Summary: {}", options.output.join("summary.csv").display());
    Ok(())
}

type Records = BTreeMap<
    (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Phase,
    ),
    Vec<Sample>,
>;

fn write_summary(path: &Path, records: &Records) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    for ((scene, view, action, variant, phase), rows) in records {
        let mut p50: Vec<_> = rows.iter().map(|row| row.p50_ms).collect();
        let mut p95: Vec<_> = rows.iter().map(|row| row.p95_ms).collect();
        let last = rows.last().context("Empty profile series")?;
        let median_p50_ms = median(&mut p50)?;
        writer.serialize(Summary {
            scene,
            view,
            action,
            variant,
            phase: *phase,
            runs: rows.len(),
            median_p50_ms,
            min_p50_ms: p50[0],
            max_p50_ms: p50[p50.len() - 1],
            median_p95_ms: median(&mut p95)?,
            shapes: last.shapes,
            vertices: last.vertices,
        })?;
    }
    writer.flush()?;
    Ok(())
}

fn write_metadata(
    options: &Options,
    cpu: usize,
    edges: usize,
    binaries: &[(&str, &PathBuf)],
) -> Result<()> {
    let mut binary_metadata = BTreeMap::new();
    for &(name, path) in binaries {
        let mut hash = Sha256::new();
        hash_file(&mut hash, path)?;
        binary_metadata.insert(
            name,
            json!({"path": path, "sha256": format!("{:x}", hash.finalize())}),
        );
    }
    let cpuinfo = fs::read_to_string("/proc/cpuinfo")?;
    let cpu_name = cpuinfo
        .lines()
        .find_map(|line| {
            line.strip_prefix("model name")
                .and_then(|line| line.split_once(':'))
                .map(|(_, name)| name.trim())
        })
        .unwrap_or("unknown");
    let governor = fs::read_to_string(format!(
        "/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor"
    ))
    .unwrap_or_else(|_| "unknown".to_owned());
    let metadata = json!({
        "cpu": cpu_name, "pinned_cpu": cpu,
        "os": command_text(Command::new("uname").arg("-a"))?,
        "governor": governor.trim(),
        "rustc": command_text(Command::new("rustc").arg("--version"))?,
        "nodes": options.nodes.get(), "edges": edges,
        "frames": options.frames.get(), "runs": options.runs.get(), "viewport": [1440, 1000], "pixels_per_point": 1,
        "binaries": binary_metadata,
        "limitations": ["CPU only, no GPU/vsync or AccessKit tree", "Background system processes were not stopped", "Both binaries must use the same deterministic scene and event harness", "Geometry changes are applied through real drag events"]
    });
    write_json(&options.output.join("metadata.json"), &metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affinity_handles_ranges_and_restricted_cpus() {
        assert_eq!(
            cpu_from_affinity("pid 1's current affinity list: 0-7").unwrap(),
            2
        );
        assert_eq!(
            cpu_from_affinity("pid 1's current affinity list: 8,10-12").unwrap(),
            8
        );
        assert!(cpu_from_affinity("invalid").is_err());
    }

    #[test]
    fn summarizes_odd_and_even_series() {
        assert_eq!(median(&mut [4.0, 1.0, 3.0]).unwrap(), 3.0);
        assert_eq!(median(&mut [4.0, 1.0, 3.0, 2.0]).unwrap(), 2.5);
        assert!(median(&mut []).is_err());
    }

    #[test]
    fn rejects_incomplete_or_incompatible_measurements() {
        let csv = "target,scene,nodes,edges,view,action,phase,samples,p50_ms,p95_ms,shapes,vertices\ncanvas,network,10,30,setup,build,geometry,1,0.5,0.5,0,0\ncanvas,network,10,30,fit,idle,ui,3,1.5,2.5,3,4\ncanvas,network,10,30,fit,idle,tessellation,3,1.5,2.5,3,4\ncanvas,network,10,30,fit,idle,cpu_total,3,1.5,2.5,3,4\n";
        let workload = Workload {
            scene: "network",
            view: "fit",
            action: "idle",
            nodes: 10,
            edges: 30,
            frames: 3,
        };
        let rows = Row::read(csv.as_bytes(), workload).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[&Phase::Ui].p50_ms, 1.5);
        for (from, to) in [
            ("1.5", "NaN"),
            ("1.5", "-1"),
            ("2.5", "1.0"),
            ("network", "orthogonal"),
            (",10,", ",11,"),
            (",30,", ",33,"),
            (",ui,3,", ",ui,2,"),
            (",tessellation,", ",ui,"),
        ] {
            assert!(
                Row::read(csv.replace(from, to).as_bytes(), workload).is_err(),
                "{from} -> {to}"
            );
        }
        let incomplete = csv.lines().take(4).collect::<Vec<_>>().join("\n");
        assert!(Row::read(incomplete.as_bytes(), workload).is_err());
    }

    #[test]
    fn refuses_to_overwrite_previous_measurements() {
        let directory = tempfile::tempdir().unwrap();
        prepare_output(directory.path()).unwrap();
        let previous = directory.path().join("summary.csv");
        fs::write(&previous, "previous measurement").unwrap();
        assert!(prepare_output(directory.path()).is_err());
        assert_eq!(
            fs::read_to_string(previous).unwrap(),
            "previous measurement"
        );
    }
}
