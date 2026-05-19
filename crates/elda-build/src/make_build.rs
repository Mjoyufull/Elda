use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use std::sync::Arc;

use crate::process::{run_command, run_command_streamed};

pub fn detect_make_build(
    _package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("Makefile").is_file() && !source_dir.join("makefile").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "make".to_owned(),
        bins: Vec::new(),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_make(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
    stream_output: bool,
    line_hook: Option<Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<(), BuildError> {
    let mut make = Command::new("make");
    make.current_dir(source_dir);
    if stream_output {
        run_command_streamed("make", make, "building make project", line_hook.clone())?;
    } else {
        run_command("make", make, "building make project")?;
    }

    if build.tests {
        let mut test = Command::new("make");
        test.current_dir(source_dir).arg("test");
        if stream_output {
            run_command_streamed("make", test, "running make tests", line_hook.clone())?;
        } else {
            run_command("make", test, "running make tests")?;
        }
    }

    let mut install = Command::new("make");
    install
        .current_dir(source_dir)
        .arg("install")
        .arg("PREFIX=/usr")
        .arg(format!("DESTDIR={}", stage_root.display()));
    if stream_output {
        run_command_streamed("make", install, "installing make project", line_hook)?;
    } else {
        run_command("make", install, "installing make project")?;
    }

    Ok(())
}
