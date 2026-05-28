use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

pub fn detect_nimble_build(
    package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if find_nimble_file(source_dir).is_none() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "nimble".to_owned(),
        bins: vec![package.name.clone()],
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_nimble(
    build: &BuildDefinition,
    package: &PackageDefinition,
    source_dir: &Path,
    stage_root: &Path,
) -> Result<(), BuildError> {
    let mut nimble = Command::new("nimble");
    nimble.current_dir(source_dir).args(["build", "-y"]);
    run_command("nimble", nimble, "building nimble project")?;

    if build.tests {
        let mut test = Command::new("nimble");
        test.current_dir(source_dir).args(["test", "-y"]);
        run_command("nimble", test, "running nimble tests")?;
    }

    let bin_dir = stage_root.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;
    let bins = if build.bins.is_empty() {
        vec![package.name.clone()]
    } else {
        build.bins.clone()
    };

    for bin in bins {
        let source_bin = find_nimble_binary(source_dir, &bin).ok_or_else(|| {
            BuildError::Invalid(format!(
                "nimble build did not produce an expected binary for `{bin}`"
            ))
        })?;
        let destination = bin_dir.join(&bin);
        fs::copy(&source_bin, &destination)?;
        let mode = fs::metadata(&source_bin)?.permissions().mode();
        fs::set_permissions(&destination, fs::Permissions::from_mode(mode | 0o755))?;
    }

    Ok(())
}

fn find_nimble_file(source_dir: &Path) -> Option<PathBuf> {
    fs::read_dir(source_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("nimble"))
}

fn find_nimble_binary(source_dir: &Path, bin: &str) -> Option<PathBuf> {
    let candidates = [
        source_dir.join(bin),
        source_dir.join("bin").join(bin),
        source_dir.join("build").join(bin),
    ];
    candidates.into_iter().find(|path| path.is_file())
}
