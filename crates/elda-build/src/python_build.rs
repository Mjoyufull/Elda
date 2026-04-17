use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::error::BuildError;
use crate::process::run_command;

pub fn detect_python_build(
    _package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("pyproject.toml").is_file() && !source_dir.join("setup.py").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "python".to_owned(),
        bins: Vec::new(),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_python(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
) -> Result<(), BuildError> {
    if source_dir.join("setup.py").is_file() {
        let record_path = source_dir.join(".elda-python-install-record.txt");
        let mut install = Command::new("python3");
        install.current_dir(source_dir).args([
            "setup.py",
            "install",
            "--root",
            &stage_root.to_string_lossy(),
            "--prefix",
            "/usr",
            "--single-version-externally-managed",
            "--record",
            &record_path.to_string_lossy(),
        ]);
        run_command("python3", install, "installing python project")?;
    } else {
        let mut install = Command::new("python3");
        install.current_dir(source_dir).args([
            "-m",
            "pip",
            "install",
            ".",
            "--no-deps",
            "--no-build-isolation",
            "--disable-pip-version-check",
            "--prefix",
            "/usr",
            "--root",
            &stage_root.to_string_lossy(),
        ]);
        run_command("python3", install, "installing python project")?;
    }
    rewrite_python_entrypoints(stage_root)?;

    if build.tests {
        let mut test = Command::new("python3");
        test.current_dir(source_dir)
            .args(["-m", "unittest", "discover"]);
        run_command("python3", test, "running python tests")?;
    }

    Ok(())
}

fn rewrite_python_entrypoints(stage_root: &Path) -> Result<(), BuildError> {
    let site_dirs = collect_python_site_dirs(stage_root)?;
    if site_dirs.is_empty() {
        return Ok(());
    }

    let bin_dir = stage_root.join("usr/bin");
    if !bin_dir.is_dir() {
        return Ok(());
    }

    let libexec_dir = stage_root.join("usr/libexec/elda/python-bin");
    fs::create_dir_all(&libexec_dir)?;

    for entry in fs::read_dir(&bin_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_python_script(&path)? {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        let runtime_path = libexec_dir.join(format!("{name}.real"));
        fs::rename(&path, &runtime_path)?;
        let original_mode = fs::metadata(&runtime_path)?.permissions().mode();
        fs::write(
            &path,
            render_python_wrapper(&name, &site_dirs, "libexec/elda/python-bin"),
        )?;
        fs::set_permissions(&path, fs::Permissions::from_mode(original_mode | 0o755))?;
    }

    Ok(())
}

fn collect_python_site_dirs(stage_root: &Path) -> Result<Vec<String>, BuildError> {
    let mut site_dirs = Vec::new();
    for lib_dir in ["lib", "lib64"] {
        let base_dir = stage_root.join("usr").join(lib_dir);
        if !base_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&base_dir)? {
            let entry = entry?;
            let version_dir = entry.path();
            if !version_dir.is_dir() {
                continue;
            }

            let Some(version_name) = version_dir.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !version_name.starts_with("python") {
                continue;
            }

            for leaf in ["site-packages", "dist-packages"] {
                if version_dir.join(leaf).is_dir() {
                    site_dirs.push(format!("{lib_dir}/{version_name}/{leaf}"));
                }
            }
        }
    }

    site_dirs.sort();
    site_dirs.dedup();
    Ok(site_dirs)
}

fn is_python_script(path: &Path) -> Result<bool, BuildError> {
    let content = fs::read(path)?;
    let head = &content[..content.len().min(256)];
    Ok(content.starts_with(b"#!")
        && head
            .windows(b"python".len())
            .any(|window| window == b"python"))
}

fn render_python_wrapper(name: &str, site_dirs: &[String], libexec_dir: &str) -> String {
    let pythonpath = site_dirs
        .iter()
        .map(|site_dir| format!("$PREFIX_DIR/{site_dir}"))
        .collect::<Vec<_>>()
        .join(":");
    let runtime_script = format!("$PREFIX_DIR/{libexec_dir}/{name}.real");

    format!(
        "#!/bin/sh\nset -eu\nPREFIX_DIR=$(CDPATH= cd -- \"$(dirname -- \"$0\")/..\" && pwd)\nELDA_PYTHONPATH=\"{pythonpath}\"\nif [ -n \"${{PYTHONPATH:-}}\" ]; then\n  export PYTHONPATH=\"$ELDA_PYTHONPATH:$PYTHONPATH\"\nelse\n  export PYTHONPATH=\"$ELDA_PYTHONPATH\"\nfi\nexec python3 \"{runtime_script}\" \"$@\"\n"
    )
}
