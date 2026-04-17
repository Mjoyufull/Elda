use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

pub fn detect_go_build(
    package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("go.mod").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "go".to_owned(),
        bins: vec![package.name.clone()],
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_go(
    build: &BuildDefinition,
    package: &PackageDefinition,
    source_dir: &Path,
    stage_root: &Path,
) -> Result<(), BuildError> {
    let bins = if build.bins.is_empty() {
        vec![package.name.clone()]
    } else {
        build.bins.clone()
    };
    let bin_dir = stage_root.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;

    for bin in &bins {
        let target = if source_dir.join("cmd").join(bin).is_dir() {
            format!("./cmd/{bin}")
        } else {
            ".".to_owned()
        };
        let destination = bin_dir.join(bin);
        let mut command = Command::new("go");
        command.current_dir(source_dir).args([
            "build",
            "-o",
            &destination.to_string_lossy(),
            &target,
        ]);
        run_command("go", command, "building go project")?;

        let mode = fs::metadata(&destination)?.permissions().mode();
        fs::set_permissions(&destination, fs::Permissions::from_mode(mode | 0o755))?;
    }

    if build.tests {
        let mut test = Command::new("go");
        test.current_dir(source_dir).args(["test", "./..."]);
        run_command("go", test, "running go tests")?;
    }

    Ok(())
}
