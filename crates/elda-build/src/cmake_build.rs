use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::BuildLineHook;
use crate::error::BuildError;
use crate::process::{run_command, run_command_inherited};

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
    line_hook: Option<BuildLineHook>,
) -> Result<(), BuildError> {
    let build_dir = source_dir.join("build-elda-cmake");
    let build_dir_arg = build_dir.to_string_lossy();
    let mut configure = Command::new("cmake");
    configure.current_dir(source_dir).args([
        "-S",
        ".",
        "-B",
        &build_dir_arg,
        "-DCMAKE_BUILD_TYPE=Release",
        "-DCMAKE_INSTALL_PREFIX=/usr",
    ]);
    if stream_output {
        emit_header(&line_hook, "cmake configure (Release)");
        run_command_inherited("cmake", configure, "configuring cmake project")?;
    } else {
        run_command("cmake", configure, "configuring cmake project")?;
    }

    let mut compile = Command::new("cmake");
    compile
        .current_dir(source_dir)
        .args(["--build", &build_dir_arg]);
    if stream_output {
        emit_header(&line_hook, "cmake --build (ninja/make backend)");
        run_command_inherited("cmake", compile, "building cmake project")?;
    } else {
        run_command("cmake", compile, "building cmake project")?;
    }

    if build.tests {
        let mut test = Command::new("ctest");
        test.current_dir(source_dir)
            .args(["--test-dir", &build_dir_arg, "--output-on-failure"]);
        if stream_output {
            emit_header(&line_hook, "ctest --output-on-failure");
            run_command_inherited("ctest", test, "running ctest")?;
        } else {
            run_command("ctest", test, "running ctest")?;
        }
    }

    let mut install = Command::new("cmake");
    install
        .current_dir(source_dir)
        .env("DESTDIR", stage_root)
        .args(["--install", &build_dir_arg]);
    if stream_output {
        emit_header(&line_hook, "cmake --install");
        run_command_inherited("cmake", install, "installing cmake project")?;
    } else {
        run_command("cmake", install, "installing cmake project")?;
    }

    Ok(())
}

fn emit_header(line_hook: &Option<BuildLineHook>, label: &str) {
    if let Some(hook) = line_hook {
        hook(label);
    }
}
