use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{io::hash_file, output, run};

const PACKAGES: [&str; 3] = ["pautinka-core", "pautinka-egui", "pautinka"];

#[derive(Deserialize)]
struct Metadata {
    target_directory: PathBuf,
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    version: String,
    manifest_path: PathBuf,
}

fn files(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            files(&path, paths)?;
        } else if path.is_file() {
            paths.push(path);
        }
    }
    Ok(())
}

fn digest(root: &Path) -> Result<String> {
    let mut inputs = vec![
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
        root.join("README.md"),
        root.join("README.ru.md"),
    ];
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with("LICENSE-") {
            inputs.push(entry.path());
        }
    }
    for directory in ["src", "crates/pautinka-core", "crates/pautinka-egui"] {
        files(&root.join(directory), &mut inputs)?;
    }
    inputs.sort();
    let mut hash = Sha256::new();
    for path in inputs {
        hash.update(
            path.strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/")
                .as_bytes(),
        );
        hash_file(&mut hash, &path)?;
    }
    let mut fingerprint = format!("{:x}", hash.finalize());
    fingerprint.truncate(16);
    Ok(fingerprint)
}

pub fn check(root: &Path, offline: bool) -> Result<()> {
    let flags: &[&str] = if offline { &["--offline"] } else { &[] };
    let metadata: Metadata = serde_json::from_slice(
        &output(
            Command::new("cargo")
                .current_dir(root)
                .args(["metadata", "--no-deps", "--format-version", "1"])
                .args(flags),
        )?
        .stdout,
    )
    .context("Failed to parse cargo metadata")?;
    let packages: BTreeMap<_, _> = metadata
        .packages
        .into_iter()
        .filter(|package| PACKAGES.contains(&package.name.as_str()))
        .map(|package| (package.name.clone(), package))
        .collect();
    ensure!(
        packages.len() == PACKAGES.len(),
        "Missing workspace packages"
    );
    let target = metadata.target_directory;
    // Cargo кеширует версии registry как неизменяемые; новый набор исходников получает новый staging registry.
    let package_target = target.join("package-check").join(digest(root)?);
    let mut package = Command::new("cargo");
    package.current_dir(root).arg("package");
    for name in PACKAGES {
        package.args(["-p", name]);
    }
    run(package
        .args(["--allow-dirty", "--target-dir"])
        .arg(&package_target)
        .args(flags))?;
    let temporary = tempfile::Builder::new()
        .prefix("pautinka-consumer-")
        .tempdir()?;
    let external = temporary.path();
    let mut extracted = BTreeMap::new();
    for name in PACKAGES {
        let directory = format!("{name}-{}", packages[name].version);
        let archive = package_target
            .join("package")
            .join(format!("{directory}.crate"));
        let path = external.join(directory);
        extract(&archive, external, &path)
            .with_context(|| format!("Failed to extract {}", archive.display()))?;
        extracted.insert(name, path);
    }
    let source = root.join("tools/consumer");
    let consumer = external.join("consumer");
    copy_consumer(&source, &consumer)?;
    prepare_consumer(&source, &consumer, &packages, &extracted)?;
    verify_consumer(&consumer, &target.join("consumer-check"), &extracted, flags)?;
    println!("All packages and the external native/WASM consumer passed with and without egui");
    println!(
        "Verified archives: {}",
        package_target.join("package").display()
    );
    Ok(())
}

fn extract(archive: &Path, destination: &Path, package: &Path) -> Result<()> {
    let mut bundle = tar::Archive::new(GzDecoder::new(fs::File::open(archive)?));
    let prefix = Path::new(
        package
            .file_name()
            .context("Missing package directory name")?,
    );
    for entry in bundle.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        let relative = path.strip_prefix(prefix)?;
        // Шаблоны Cargo без начального слеша могут захватить README из tools и других каталогов.
        ensure!(
            relative.starts_with("src")
                || relative.starts_with("examples")
                || matches!(
                    relative.to_str(),
                    Some(
                        "Cargo.toml"
                            | "Cargo.toml.orig"
                            | "Cargo.lock"
                            | ".cargo_vcs_info.json"
                            | "README.md"
                            | "README.ru.md"
                            | "LICENSE-MIT"
                            | "LICENSE-APACHE"
                    )
                ),
            "Unexpected file in package: {}",
            path.display()
        );
        ensure!(entry.unpack_in(destination)?, "Unsafe archive path");
    }
    for required in ["Cargo.toml", "README.md", "LICENSE-MIT", "LICENSE-APACHE"] {
        ensure!(package.join(required).is_file(), "Missing {required}");
    }
    Ok(())
}

fn prepare_consumer(
    source: &Path,
    consumer: &Path,
    packages: &BTreeMap<String, Package>,
    extracted: &BTreeMap<&str, PathBuf>,
) -> Result<()> {
    let manifest_path = consumer.join("Cargo.toml");
    let mut manifest: toml::Value = toml::from_str(&fs::read_to_string(&manifest_path)?)?;
    for name in PACKAGES {
        let original = packages[name]
            .manifest_path
            .parent()
            .context("Missing package directory")?;
        // Меняем только path-зависимости, сохраняя версии и настройки consumer.
        if let Some(table) = manifest.get_mut("dependencies") {
            rewrite_paths(table, source, original, &extracted[name])?;
        }
        if let Some(table) = manifest
            .get_mut("patch")
            .and_then(|patch| patch.get_mut("crates-io"))
        {
            rewrite_paths(table, source, original, &extracted[name])?;
        }
    }
    fs::write(&manifest_path, toml::to_string(&manifest)?)?;
    Ok(())
}

fn verify_consumer(
    consumer: &Path,
    target: &Path,
    extracted: &BTreeMap<&str, PathBuf>,
    flags: &[&str],
) -> Result<()> {
    let cargo = || {
        let mut command = Command::new("cargo");
        command
            .current_dir(consumer)
            .env("CARGO_TARGET_DIR", target);
        command
    };
    run(cargo().args(["run", "--no-default-features"]).args(flags))?;
    let tree = output(
        cargo()
            .args([
                "tree",
                "--no-default-features",
                "--edges",
                "normal,build",
                "--prefix",
                "none",
                "--format",
                "{p}",
            ])
            .args(flags),
    )?;
    ensure!(
        !String::from_utf8(tree.stdout)?
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .any(|name| matches!(name, "egui" | "pautinka-egui")),
        "Geometry-only consumer must not depend on egui"
    );
    run(cargo().arg("run").args(flags))?;
    for features in [&["--no-default-features"][..], &[][..]] {
        run(cargo()
            .arg("check")
            .args(features)
            .args(["--target", "wasm32-unknown-unknown"])
            .args(flags))?;
    }
    let patch = format!(
        "patch.crates-io.pautinka-core.path={}",
        toml::Value::String(extracted["pautinka-core"].to_string_lossy().into_owned())
    );
    run(Command::new("cargo")
        .current_dir(&extracted["pautinka-egui"])
        .env("CARGO_TARGET_DIR", target)
        .args(["check", "--examples", "--config", &patch])
        .args(flags))?;
    Ok(())
}

fn rewrite_paths(
    table: &mut toml::Value,
    source: &Path,
    original: &Path,
    extracted: &Path,
) -> Result<()> {
    for (_, dependency) in table
        .as_table_mut()
        .context("Expected dependency table")?
        .iter_mut()
    {
        if let Some(path) = dependency.get_mut("path")
            && source
                .join(path.as_str().context("Expected dependency path")?)
                .canonicalize()?
                == original
        {
            *path = toml::Value::String(
                extracted
                    .to_str()
                    .context("Non-UTF-8 package path")?
                    .to_owned(),
            );
        }
    }
    Ok(())
}

fn copy_consumer(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.file_name() == "target" || entry.file_name() == "Cargo.lock" {
            continue;
        }
        let path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_consumer(&entry.path(), &path)?;
        } else {
            fs::copy(entry.path(), path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_readme_leaked_from_tools_into_archive() {
        let temporary = tempfile::tempdir().unwrap();
        let package = temporary.path().join("pautinka-0.1.0");
        for extra in [None, Some("tools/README.md")] {
            let archive = temporary.path().join("test.crate");
            let encoder = flate2::write::GzEncoder::new(
                fs::File::create(&archive).unwrap(),
                flate2::Compression::default(),
            );
            let mut bundle = tar::Builder::new(encoder);
            for path in [
                "Cargo.toml",
                "README.md",
                "LICENSE-MIT",
                "LICENSE-APACHE",
                "src/lib.rs",
            ]
            .into_iter()
            .chain(extra)
            {
                let mut header = tar::Header::new_gnu();
                header.set_size(0);
                header.set_mode(0o644);
                header.set_cksum();
                bundle
                    .append_data(&mut header, format!("pautinka-0.1.0/{path}"), &[][..])
                    .unwrap();
            }
            bundle.into_inner().unwrap().finish().unwrap();
            let result = extract(&archive, temporary.path(), &package);
            if extra.is_some() {
                assert!(
                    result
                        .unwrap_err()
                        .to_string()
                        .contains("Unexpected file in package: pautinka-0.1.0/tools/README.md")
                );
            } else {
                result.unwrap();
            }
        }
    }

    #[test]
    fn redirects_only_matching_local_dependencies() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("consumer");
        let original = root.path().join("library");
        let other = root.path().join("other");
        for path in [&source, &original, &other] {
            fs::create_dir(path).unwrap();
        }
        let mut dependencies: toml::Value = toml::from_str(
            r#"
registry = "1"
local = { path = "../library", version = "0.1.0", optional = true }
other = { path = "../other" }
"#,
        )
        .unwrap();
        let extracted = root.path().join("extracted");
        rewrite_paths(
            &mut dependencies,
            &source,
            &original.canonicalize().unwrap(),
            &extracted,
        )
        .unwrap();
        assert_eq!(
            dependencies["local"]["path"].as_str().unwrap(),
            extracted.to_str().unwrap()
        );
        assert_eq!(dependencies["local"]["version"].as_str().unwrap(), "0.1.0");
        assert_eq!(dependencies["local"]["optional"].as_bool(), Some(true));
        assert_eq!(dependencies["other"]["path"].as_str().unwrap(), "../other");
        assert_eq!(dependencies["registry"].as_str().unwrap(), "1");
    }
}
