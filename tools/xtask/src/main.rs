use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};

mod check;
mod io;
mod metro;
mod packaging;
mod profiling;
mod release;

#[derive(Parser)]
#[command(about = "Repository maintenance tasks")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    #[command(about = "Run native checks; --full also checks docs, WASM and archives.")]
    Check {
        #[arg(long)]
        full: bool,
        #[arg(long)]
        offline: bool,
    },
    #[command(about = "Rebuild the Moscow demo map from source JSON.")]
    BuildMsk,
    #[command(about = "Verify Cargo archives and an independent native/WASM consumer.")]
    PackageCheck {
        #[arg(long)]
        offline: bool,
    },
    #[command(about = "Compare two prebuilt CPU profiling binaries (Linux, requires taskset).")]
    Compare(profiling::Options),
    #[command(about = "Check a release tag against workspace and dependency versions.")]
    CheckRelease {
        #[arg(env = "GITHUB_REF_NAME")]
        tag: String,
    },
}

fn root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .context("Failed to locate workspace")
}

fn output(command: &mut Command) -> Result<Output> {
    let output = command
        .output()
        .with_context(|| format!("Failed to execute {command:?}"))?;
    ensure!(
        output.status.success(),
        "Command failed ({}): {command:?}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("Failed to execute {command:?}"))?;
    ensure!(status.success(), "Command failed ({status}): {command:?}");
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().task {
        Task::Check { full, offline } => check::check(&root()?, full, offline),
        Task::BuildMsk => metro::write(&root()?.join("demos/metro/data")),
        Task::PackageCheck { offline } => packaging::check(&root()?, offline),
        Task::Compare(options) => profiling::compare(&options),
        Task::CheckRelease { tag } => release::check(&root()?, &tag),
    }
}
