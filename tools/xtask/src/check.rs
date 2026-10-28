use std::{path::Path, process::Command};

use anyhow::Result;

use crate::{packaging, run};

pub fn check(root: &Path, full: bool, offline: bool) -> Result<()> {
    let cargo = || {
        let mut command = Command::new("cargo");
        command.current_dir(root);
        if offline {
            command.arg("--offline");
        }
        command
    };
    run(cargo().args(["fmt", "--all", "--", "--check"]))?;
    run(cargo().args([
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ]))?;
    run(cargo().args(["test", "--workspace", "--all-features"]))?;
    run(cargo().args(["test", "-p", "pautinka", "--no-default-features"]))?;
    if full {
        run(cargo().env("RUSTDOCFLAGS", "-D warnings").args([
            "doc",
            "-p",
            "pautinka",
            "-p",
            "pautinka-egui",
            "-p",
            "pautinka-core",
            "--all-features",
            "--no-deps",
        ]))?;
        run(cargo().args([
            "check",
            "-p",
            "pautinka",
            "-p",
            "pautinka-egui",
            "-p",
            "pautinka-core",
            "--all-features",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
        ]))?;
        run(cargo().args([
            "check",
            "-p",
            "pautinka-demo",
            "-p",
            "pautinka-stress",
            "--target",
            "wasm32-unknown-unknown",
        ]))?;
        packaging::check(root, offline)?;
    }
    Ok(())
}
