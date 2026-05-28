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
        validate_bin_name(bin)?;
        let target = if source_dir.join("cmd").join(bin).is_dir() {
            format!("./cmd/{bin}")
        } else {
            ".".to_owned()
        };
        let destination = bin_dir.join(bin);
        let mut command = Command::new("go");
        command
            .current_dir(source_dir)
            .arg("build")
            .arg("-o")
            .arg(&destination)
            .arg(&target);
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

fn validate_bin_name(value: &str) -> Result<(), BuildError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
    {
        return Err(BuildError::Invalid(format!(
            "go build binary name `{value}` must be a plain file name"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_bin_name;

    #[test]
    fn go_bin_name_must_not_escape_stage_bin_dir() {
        validate_bin_name("demo").expect("plain name should pass");
        validate_bin_name("../demo").expect_err("relative escape should fail");
        validate_bin_name("/tmp/demo").expect_err("absolute path should fail");
    }
}
