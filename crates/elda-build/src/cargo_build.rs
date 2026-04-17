use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

pub fn detect_cargo_build(
    package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("Cargo.toml").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "cargo".to_owned(),
        bins: cargo_bins(source_dir)?.unwrap_or_else(|| vec![package.name.clone()]),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_cargo(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
) -> Result<(), BuildError> {
    let bins = if build.bins.is_empty() {
        cargo_bins(source_dir)?.unwrap_or_default()
    } else {
        build.bins.clone()
    };

    let mut build_command = Command::new("cargo");
    build_command
        .current_dir(source_dir)
        .args(["build", "--release"]);
    if !build.features.is_empty() {
        build_command.arg("--features");
        build_command.arg(build.features.join(","));
    }
    for bin in &bins {
        build_command.args(["--bin", bin]);
    }
    run_command("cargo", build_command, "building cargo project")?;

    if build.tests {
        let mut test_command = Command::new("cargo");
        test_command
            .current_dir(source_dir)
            .args(["test", "--release"]);
        if !build.features.is_empty() {
            test_command.arg("--features");
            test_command.arg(build.features.join(","));
        }
        run_command("cargo", test_command, "running cargo tests")?;
    }

    let target_dir = target_directory(source_dir)?;
    let bin_dir = stage_root.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;

    for bin in bins {
        let source_bin = target_dir.join("release").join(&bin);
        if !source_bin.is_file() {
            return Err(BuildError::Invalid(format!(
                "expected cargo build to produce `{}`",
                source_bin.display()
            )));
        }
        let destination = bin_dir.join(&bin);
        fs::copy(&source_bin, &destination)?;
        let mode = fs::metadata(&source_bin)?.permissions().mode();
        fs::set_permissions(&destination, fs::Permissions::from_mode(mode))?;
    }

    Ok(())
}

fn cargo_bins(source_dir: &Path) -> Result<Option<Vec<String>>, BuildError> {
    let metadata = cargo_metadata(source_dir)?;
    let mut bins = metadata
        .packages
        .into_iter()
        .find(|package| package.manifest_path == source_dir.join("Cargo.toml"))
        .map(|package| {
            package
                .targets
                .into_iter()
                .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
                .map(|target| target.name)
                .collect::<Vec<_>>()
        });

    if let Some(values) = &mut bins {
        values.sort();
        values.dedup();
    }

    Ok(bins.filter(|values| !values.is_empty()))
}

fn target_directory(source_dir: &Path) -> Result<PathBuf, BuildError> {
    let metadata = cargo_metadata(source_dir)?;
    Ok(metadata.target_directory)
}

fn cargo_metadata(source_dir: &Path) -> Result<CargoMetadata, BuildError> {
    let output = Command::new("cargo")
        .current_dir(source_dir)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(BuildError::CommandFailed {
            program: "cargo",
            context: "reading cargo metadata".to_owned(),
            stderr: if stderr.is_empty() {
                "command exited with a non-zero status".to_owned()
            } else {
                stderr
            },
        });
    }

    let metadata = serde_json::from_slice::<CargoMetadata>(&output.stdout)?;
    Ok(metadata)
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    target_directory: PathBuf,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    manifest_path: PathBuf,
    targets: Vec<CargoTarget>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}
