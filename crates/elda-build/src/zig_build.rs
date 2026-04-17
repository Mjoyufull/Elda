use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

pub fn detect_zig_build(
    _package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("build.zig").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "zig".to_owned(),
        bins: Vec::new(),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_zig(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
) -> Result<(), BuildError> {
    let prefix = stage_root.join("usr");
    let mut install = Command::new("zig");
    install.current_dir(source_dir).args([
        "build",
        "install",
        "-Doptimize=ReleaseSafe",
        "--prefix",
        &prefix.to_string_lossy(),
    ]);
    run_command("zig", install, "building zig project")?;

    if build.tests {
        let mut test = Command::new("zig");
        test.current_dir(source_dir)
            .args(["build", "test", "-Doptimize=ReleaseSafe"]);
        run_command("zig", test, "running zig tests")?;
    }

    Ok(())
}
