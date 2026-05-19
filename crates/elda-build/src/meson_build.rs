use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use std::sync::Arc;

use crate::process::{run_command, run_command_streamed};

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
    stream_output: bool,
    line_hook: Option<Arc<dyn Fn(&str) + Send + Sync>>,
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
    if stream_output {
        run_command_streamed(
            "meson",
            setup,
            "configuring meson project",
            line_hook.clone(),
        )?;
    } else {
        run_command("meson", setup, "configuring meson project")?;
    }

    let mut compile = Command::new("meson");
    compile
        .current_dir(source_dir)
        .args(["compile", "-C", &build_dir.to_string_lossy()]);
    if stream_output {
        run_command_streamed(
            "meson",
            compile,
            "building meson project",
            line_hook.clone(),
        )?;
    } else {
        run_command("meson", compile, "building meson project")?;
    }

    if build.tests {
        let mut test = Command::new("meson");
        test.current_dir(source_dir)
            .args(["test", "-C", &build_dir.to_string_lossy()]);
        if stream_output {
            run_command_streamed("meson", test, "running meson tests", line_hook.clone())?;
        } else {
            run_command("meson", test, "running meson tests")?;
        }
    }

    let mut install = Command::new("meson");
    install.current_dir(source_dir).args([
        "install",
        "-C",
        &build_dir.to_string_lossy(),
        "--destdir",
        &stage_root.to_string_lossy(),
    ]);
    if stream_output {
        run_command_streamed("meson", install, "installing meson project", line_hook)?;
    } else {
        run_command("meson", install, "installing meson project")?;
    }

    Ok(())
}
