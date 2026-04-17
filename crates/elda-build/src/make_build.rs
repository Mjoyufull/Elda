use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

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
) -> Result<(), BuildError> {
    let mut make = Command::new("make");
    make.current_dir(source_dir);
    run_command("make", make, "building make project")?;

    if build.tests {
        let mut test = Command::new("make");
        test.current_dir(source_dir).arg("test");
        run_command("make", test, "running make tests")?;
    }

    let mut install = Command::new("make");
    install
        .current_dir(source_dir)
        .arg("install")
        .arg("PREFIX=/usr")
        .arg(format!("DESTDIR={}", stage_root.display()));
    run_command("make", install, "installing make project")?;

    Ok(())
}
