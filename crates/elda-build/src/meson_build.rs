use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::{run_command, run_command_inherited};

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
    setup
        .current_dir(source_dir)
        .arg("setup")
        .arg(&build_dir)
        .args(["--prefix", "/usr", "--buildtype", "release"]);
    if stream_output {
        emit_header(&line_hook, "meson setup --buildtype release");
        run_command_inherited("meson", setup, "configuring meson project")?;
    } else {
        run_command("meson", setup, "configuring meson project")?;
    }

    let mut compile = Command::new("meson");
    compile
        .current_dir(source_dir)
        .args(["compile", "-C"])
        .arg(&build_dir);
    if stream_output {
        emit_header(&line_hook, "meson compile (ninja backend)");
        run_command_inherited("meson", compile, "building meson project")?;
    } else {
        run_command("meson", compile, "building meson project")?;
    }

    if build.tests {
        let mut test = Command::new("meson");
        test.current_dir(source_dir)
            .args(["test", "-C"])
            .arg(&build_dir);
        if stream_output {
            emit_header(&line_hook, "meson test");
            run_command_inherited("meson", test, "running meson tests")?;
        } else {
            run_command("meson", test, "running meson tests")?;
        }
    }

    let mut install = Command::new("meson");
    install
        .current_dir(source_dir)
        .args(["install", "-C"])
        .arg(&build_dir)
        .arg("--destdir")
        .arg(stage_root);
    if stream_output {
        emit_header(&line_hook, "meson install");
        run_command_inherited("meson", install, "installing meson project")?;
    } else {
        run_command("meson", install, "installing meson project")?;
    }

    Ok(())
}

fn emit_header(line_hook: &Option<Arc<dyn Fn(&str) + Send + Sync>>, label: &str) {
    if let Some(hook) = line_hook {
        hook(label);
    }
}
