use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::{run_command, run_command_streamed};

pub fn detect_cmake_build(
    _package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("CMakeLists.txt").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "cmake".to_owned(),
        bins: Vec::new(),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_cmake(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
    stream_output: bool,
    line_hook: Option<Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<(), BuildError> {
    let build_dir = source_dir.join("build-elda-cmake");
    let mut configure = Command::new("cmake");
    configure.current_dir(source_dir).args([
        "-S",
        ".",
        "-B",
        &build_dir.to_string_lossy(),
        "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_INSTALL_PREFIX=/usr",
    ]);
    if stream_output {
        run_command_streamed(
            "cmake",
            configure,
            "configuring cmake project",
            line_hook.clone(),
        )?;
    } else {
        run_command("cmake", configure, "configuring cmake project")?;
    }

    let mut compile = Command::new("cmake");
    compile
        .current_dir(source_dir)
        .args(["--build", &build_dir.to_string_lossy()]);
    if stream_output {
        run_command_streamed(
            "cmake",
            compile,
            "building cmake project",
            line_hook.clone(),
        )?;
    } else {
        run_command("cmake", compile, "building cmake project")?;
    }

    if build.tests {
        let mut test = Command::new("ctest");
        test.current_dir(source_dir).args([
            "--test-dir",
            &build_dir.to_string_lossy(),
            "--output-on-failure",
        ]);
        if stream_output {
            run_command_streamed("cmake", test, "running ctest", line_hook.clone())?;
        } else {
            run_command("ctest", test, "running ctest")?;
        }
    }

    let mut install = Command::new("cmake");
    install
        .current_dir(source_dir)
        .env("DESTDIR", stage_root)
        .args(["--install", &build_dir.to_string_lossy()]);
    if stream_output {
        run_command_streamed("cmake", install, "installing cmake project", line_hook)?;
    } else {
        run_command("cmake", install, "installing cmake project")?;
    }

    Ok(())
}
