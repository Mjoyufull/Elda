use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

pub fn detect_meson_build(
    _package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("meson.build").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "meson".to_owned(),
        bins: Vec::new(),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_meson(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
) -> Result<(), BuildError> {
    let build_dir = source_dir.join("build-elda-meson");
    let mut setup = Command::new("meson");
    setup.current_dir(source_dir).args([
        "setup",
        &build_dir.to_string_lossy(),
        "--prefix",
        "/usr",
        "--buildtype",
        "release",
    ]);
    run_command("meson", setup, "configuring meson project")?;

    let mut compile = Command::new("meson");
    compile
        .current_dir(source_dir)
        .args(["compile", "-C", &build_dir.to_string_lossy()]);
    run_command("meson", compile, "building meson project")?;

    if build.tests {
        let mut test = Command::new("meson");
        test.current_dir(source_dir)
            .args(["test", "-C", &build_dir.to_string_lossy()]);
        run_command("meson", test, "running meson tests")?;
    }

    let mut install = Command::new("meson");
    install.current_dir(source_dir).args([
        "install",
        "-C",
        &build_dir.to_string_lossy(),
        "--destdir",
        &stage_root.to_string_lossy(),
    ]);
    run_command("meson", install, "installing meson project")?;

    Ok(())
}
