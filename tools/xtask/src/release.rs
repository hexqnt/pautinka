use std::path::Path;

use anyhow::{Context, Result, ensure};
use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    workspace: Workspace,
}

#[derive(Deserialize)]
struct Workspace {
    package: Package,
    dependencies: Dependencies,
}

#[derive(Deserialize)]
struct Dependencies {
    #[serde(rename = "pautinka-core")]
    core: Package,
    #[serde(rename = "pautinka-egui")]
    egui: Package,
}

#[derive(Deserialize)]
struct Package {
    version: String,
}

fn verify(manifest: &str, tag: &str) -> Result<()> {
    let manifest: Manifest = toml::from_str(manifest)?;
    let version = &manifest.workspace.package.version;
    ensure!(
        tag.strip_prefix('v') == Some(version.as_str()),
        "Tag {tag:?} must match workspace version v{version}"
    );
    for (name, dependency) in [
        ("pautinka-core", &manifest.workspace.dependencies.core),
        ("pautinka-egui", &manifest.workspace.dependencies.egui),
    ] {
        ensure!(
            dependency.version == *version,
            "{name} dependency version must be {version}, got {}",
            dependency.version
        );
    }
    Ok(())
}

pub fn check(root: &Path, tag: &str) -> Result<()> {
    let path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    verify(&manifest, tag).with_context(|| format!("Invalid release manifest {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mismatched_tags_and_dependencies() {
        let manifest = r#"
[workspace.package]
version = "0.1.0"
[workspace.dependencies]
pautinka-core = { path = "crates/pautinka-core", version = "0.1.0" }
pautinka-egui = { path = "crates/pautinka-egui", version = "0.1.0" }
egui = "0.37"
"#;
        assert!(verify(manifest, "v0.1.0").is_ok());
        assert!(verify(manifest, "0.1.0").is_err());
        for name in ["pautinka-core", "pautinka-egui"] {
            let mismatched = manifest.replace(
                &format!("path = \"crates/{name}\", version = \"0.1.0\""),
                &format!("path = \"crates/{name}\", version = \"0.2.0\""),
            );
            assert!(verify(&mismatched, "v0.1.0").is_err(), "{name}");
        }
    }
}
