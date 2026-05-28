use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use elda_recipe::{BuildDefinition, PackageDefinition};

use crate::BuildLineHook;
use crate::error::BuildError;
use crate::process::{command_failure_message, run_command, run_command_inherited};

pub fn detect_cargo_build(
    package: &PackageDefinition,
    source_dir: &Path,
) -> Result<Option<BuildDefinition>, BuildError> {
    if !source_dir.join("Cargo.toml").is_file() {
        return Ok(None);
    }

    Ok(Some(BuildDefinition {
        system: "cargo".to_owned(),
        bins: cargo_bins(source_dir)?.unwrap_or_else(|| vec![package.name.clone()]),
        features: Vec::new(),
        tests: false,
    }))
}

pub fn build_with_cargo(
    build: &BuildDefinition,
    source_dir: &Path,
    stage_root: &Path,
    stream_output: bool,
    line_hook: Option<BuildLineHook>,
) -> Result<(), BuildError> {
    let bins = resolve_cargo_bins(build, source_dir)?;
    let runner = cargo_runner(source_dir)?;

    let mut build_command = runner.command();
    build_command
        .current_dir(source_dir)
        .args(["build", "--release"]);
    if !build.features.is_empty() {
        build_command.arg("--features");
        build_command.arg(build.features.join(","));
    }
    for bin in &bins {
        build_command.args(["--bin", bin]);
    }
    if stream_output {
        if let Some(hook) = &line_hook {
            hook("cargo build --release");
        }
        run_command_inherited("cargo", build_command, "building cargo project")?;
    } else {
        run_command("cargo", build_command, "building cargo project")?;
    }

    if build.tests {
        let mut test_command = runner.command();
        test_command
            .current_dir(source_dir)
            .args(["test", "--release"]);
        if !build.features.is_empty() {
            test_command.arg("--features");
            test_command.arg(build.features.join(","));
        }
        if stream_output {
            if let Some(hook) = &line_hook {
                hook("cargo test --release");
            }
            run_command_inherited("cargo", test_command, "running cargo tests")?;
        } else {
            run_command("cargo", test_command, "running cargo tests")?;
        }
    }

    let target_dir = target_directory(source_dir)?;
    let bin_dir = stage_root.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;

    for bin in bins {
        let source_bin = target_dir.join("release").join(&bin);
        if !source_bin.is_file() {
            return Err(BuildError::Invalid(format!(
                "expected cargo build to produce `{}`",
                source_bin.display()
            )));
        }
        let destination = bin_dir.join(&bin);
        fs::copy(&source_bin, &destination)?;
        let mode = fs::metadata(&source_bin)?.permissions().mode();
        fs::set_permissions(&destination, fs::Permissions::from_mode(mode))?;
    }

    Ok(())
}

fn resolve_cargo_bins(
    build: &BuildDefinition,
    source_dir: &Path,
) -> Result<Vec<String>, BuildError> {
    let detected = cargo_bins(source_dir)?.unwrap_or_default();
    if build.bins.is_empty() || detected.is_empty() {
        return Ok(if build.bins.is_empty() {
            detected
        } else {
            build.bins.clone()
        });
    }

    build
        .bins
        .iter()
        .map(|requested| resolve_cargo_bin_name(requested, &detected))
        .collect()
}

fn resolve_cargo_bin_name(requested: &str, detected: &[String]) -> Result<String, BuildError> {
    if detected.iter().any(|name| name == requested) {
        return Ok(requested.to_owned());
    }

    let matches = detected
        .iter()
        .filter(|name| name.eq_ignore_ascii_case(requested))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [name] => Ok((*name).clone()),
        [] => Err(BuildError::Invalid(format!(
            "cargo binary target `{requested}` was requested, but Cargo.toml exposes {}",
            detected.join(", ")
        ))),
        _ => Err(BuildError::Invalid(format!(
            "cargo binary target `{requested}` is ambiguous; Cargo.toml exposes {}",
            detected.join(", ")
        ))),
    }
}

fn workspace_manifest_path(source_dir: &Path) -> PathBuf {
    source_dir.join("Cargo.toml")
}

fn collect_package_bins(package: &CargoPackage) -> Vec<String> {
    package
        .targets
        .iter()
        .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
        .map(|target| target.name.clone())
        .collect()
}

fn sorted_unique_bins(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names.dedup();
    names
}

fn package_is_workspace_root(package: &CargoPackage, source_dir: &Path) -> bool {
    let expected = workspace_manifest_path(source_dir);
    package.manifest_path == expected
        || match (
            package.manifest_path.canonicalize(),
            expected.canonicalize(),
        ) {
            (Ok(lhs), Ok(rhs)) => lhs == rhs,
            _ => false,
        }
}

fn cargo_bins(source_dir: &Path) -> Result<Option<Vec<String>>, BuildError> {
    let metadata = cargo_metadata(source_dir)?;

    // Non-virtual workspace / single crate: bins live on the root package.
    if let Some(root_package) = metadata
        .packages
        .iter()
        .find(|package| package_is_workspace_root(package, source_dir))
    {
        let names = collect_package_bins(root_package);
        if !names.is_empty() {
            return Ok(Some(sorted_unique_bins(names)));
        }
    }

    // Virtual workspace (root Cargo.toml has `[workspace]` only): there is no root package in
    // `cargo metadata` output (see Cargo book / `workspace_members`). Collect bin targets from
    // default members — same selection `cargo build` at the workspace root uses.
    let member_ids: Vec<&str> = if !metadata.workspace_default_members.is_empty() {
        metadata
            .workspace_default_members
            .iter()
            .map(String::as_str)
            .collect()
    } else {
        metadata
            .workspace_members
            .iter()
            .map(String::as_str)
            .collect()
    };

    if member_ids.is_empty() {
        return Ok(None);
    }

    let member_set: HashSet<&str> = member_ids.into_iter().collect();
    let names: Vec<String> = metadata
        .packages
        .iter()
        .filter(|package| member_set.contains(package.id.as_str()))
        .flat_map(collect_package_bins)
        .collect();

    if names.is_empty() {
        return Ok(None);
    }

    Ok(Some(sorted_unique_bins(names)))
}

fn target_directory(source_dir: &Path) -> Result<PathBuf, BuildError> {
    let metadata = cargo_metadata(source_dir)?;
    Ok(metadata.target_directory)
}

fn cargo_metadata(source_dir: &Path) -> Result<CargoMetadata, BuildError> {
    let runner = cargo_runner(source_dir)?;
    let output = cargo_output(
        &runner,
        source_dir,
        &["metadata", "--format-version", "1", "--no-deps"],
        "reading cargo metadata",
    )?;
    let metadata = serde_json::from_slice::<CargoMetadata>(&output.stdout)?;
    Ok(metadata)
}

fn cargo_runner(source_dir: &Path) -> Result<CargoRunner, BuildError> {
    let direct_runner = CargoRunner::cargo();
    match cargo_output(
        &direct_runner,
        source_dir,
        &["metadata", "--format-version", "1", "--no-deps"],
        "reading cargo metadata",
    ) {
        Ok(_) => Ok(direct_runner),
        Err(error) if missing_rustup_default(&error) => fallback_cargo_runner(source_dir),
        Err(error) => Err(error),
    }
}

fn fallback_cargo_runner(source_dir: &Path) -> Result<CargoRunner, BuildError> {
    let requested = requested_toolchain(source_dir).or_else(env_requested_toolchain);

    current_home_toolchain_runner(requested.as_deref())
        .or_else(|| invoking_user_toolchain_runner(requested.as_deref()))
        .ok_or_else(|| {
            BuildError::Unsupported(
                "cargo is managed by rustup, but no default toolchain is configured and Elda could not infer a usable fallback toolchain; add `rust-toolchain.toml`, export `RUSTUP_TOOLCHAIN`, or run `rustup default stable`".to_owned(),
            )
        })
}

fn cargo_output(
    runner: &CargoRunner,
    source_dir: &Path,
    args: &[&str],
    context: &str,
) -> Result<std::process::Output, BuildError> {
    let mut command = runner.command();
    command.current_dir(source_dir).args(args);
    let output = command.output()?;
    if output.status.success() {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Err(BuildError::CommandFailed {
        program: "cargo",
        context: if runner.description == "cargo" {
            context.to_owned()
        } else {
            format!("{context} via `{}`", runner.description)
        },
        stderr: command_failure_message("cargo", &stderr, &stdout),
    })
}

fn requested_toolchain(source_dir: &Path) -> Option<String> {
    requested_toolchain_toml(source_dir).or_else(|| requested_toolchain_file(source_dir))
}

fn env_requested_toolchain() -> Option<String> {
    let value = env::var("RUSTUP_TOOLCHAIN").ok()?;
    sanitize_toolchain_name(value)
}

fn requested_toolchain_toml(source_dir: &Path) -> Option<String> {
    let path = source_dir.join("rust-toolchain.toml");
    let content = fs::read_to_string(path).ok()?;
    let parsed = toml::from_str::<RustToolchainToml>(&content).ok()?;
    let channel = parsed.toolchain?.channel?.trim().to_owned();
    (!channel.is_empty()).then_some(channel)
}

fn requested_toolchain_file(source_dir: &Path) -> Option<String> {
    let path = source_dir.join("rust-toolchain");
    let content = fs::read_to_string(path).ok()?;
    parse_toolchain_text(&content)
}

fn parse_toolchain_text(content: &str) -> Option<String> {
    if let Ok(parsed) = toml::from_str::<RustToolchainToml>(content)
        && let Some(channel) = parsed.toolchain.and_then(|toolchain| toolchain.channel)
        && let Some(channel) = sanitize_toolchain_name(channel)
    {
        return Some(channel);
    }

    content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('['))
        .and_then(|line| sanitize_toolchain_name(line.to_owned()))
}

fn sanitize_toolchain_name(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "no installed toolchains" {
        return None;
    }

    let normalized = trimmed
        .strip_prefix("channel = ")
        .unwrap_or(trimmed)
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    if normalized.is_empty() {
        return None;
    }

    Some(normalized.to_owned())
}

fn current_home_toolchain_runner(requested: Option<&str>) -> Option<CargoRunner> {
    let rustup_home = current_rustup_home()?;
    toolchain_runner_from_home(&rustup_home, requested, "current rustup home")
}

fn invoking_user_toolchain_runner(requested: Option<&str>) -> Option<CargoRunner> {
    let rustup_home = invoking_user_rustup_home()?;
    toolchain_runner_from_home(&rustup_home, requested, "invoking user rustup home")
}

fn current_rustup_home() -> Option<PathBuf> {
    env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".rustup")))
        .filter(|path| path.join("toolchains").is_dir())
}

fn invoking_user_rustup_home() -> Option<PathBuf> {
    let user = env::var("SUDO_USER")
        .ok()
        .or_else(|| env::var("DOAS_USER").ok())?;
    let home = home_dir_for_user(&user)?;
    let rustup_home = home.join(".rustup");
    rustup_home
        .join("toolchains")
        .is_dir()
        .then_some(rustup_home)
}

fn home_dir_for_user(user: &str) -> Option<PathBuf> {
    let passwd = fs::read_to_string("/etc/passwd").ok()?;
    passwd.lines().find_map(|line| {
        let mut parts = line.split(':');
        let name = parts.next()?;
        if name != user {
            return None;
        }
        let _password = parts.next()?;
        let _uid = parts.next()?;
        let _gid = parts.next()?;
        let _gecos = parts.next()?;
        let home = parts.next()?;
        Some(PathBuf::from(home))
    })
}

fn toolchain_runner_from_home(
    rustup_home: &Path,
    requested: Option<&str>,
    home_label: &str,
) -> Option<CargoRunner> {
    let toolchains = installed_toolchains_in_home(rustup_home);
    let selected = select_toolchain_name(requested, &toolchains)
        .or_else(|| preferred_stable_toolchain(&toolchains))
        .or_else(|| toolchains.first().cloned())?;
    let cargo_path = rustup_home
        .join("toolchains")
        .join(&selected)
        .join("bin/cargo");
    let rustc_dir = cargo_path.parent()?.to_path_buf();
    cargo_path.is_file().then(|| {
        CargoRunner::toolchain_binary(
            cargo_path,
            rustc_dir,
            format!("{home_label} toolchain `{selected}` cargo"),
        )
    })
}

fn installed_toolchains_in_home(rustup_home: &Path) -> Vec<String> {
    let mut toolchains = fs::read_dir(rustup_home.join("toolchains"))
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry.file_name().to_string_lossy().into_owned())
        })
        .collect::<Vec<_>>();
    toolchains.sort();
    toolchains
}

fn select_toolchain_name(requested: Option<&str>, installed: &[String]) -> Option<String> {
    let requested = requested?;
    installed
        .iter()
        .find(|toolchain| toolchain.as_str() == requested)
        .cloned()
        .or_else(|| {
            ["stable", "beta", "nightly"]
                .iter()
                .find(|channel| requested == **channel)
                .and_then(|channel| {
                    installed
                        .iter()
                        .find(|toolchain| toolchain.starts_with(&format!("{channel}-")))
                        .cloned()
                })
        })
}

fn preferred_stable_toolchain(installed: &[String]) -> Option<String> {
    installed
        .iter()
        .find(|toolchain| toolchain.as_str() == "stable" || toolchain.starts_with("stable-"))
        .cloned()
}

fn missing_rustup_default(error: &BuildError) -> bool {
    match error {
        BuildError::CommandFailed { stderr, .. } => {
            stderr.contains("rustup could not choose a version of cargo to run")
                && stderr.contains("no default is configured")
        }
        _ => false,
    }
}

#[derive(Debug, Clone)]
struct CargoRunner {
    program: OsString,
    args: Vec<OsString>,
    description: String,
    toolchain_bin_dir: Option<PathBuf>,
}

impl CargoRunner {
    fn cargo() -> Self {
        Self {
            program: OsString::from("cargo"),
            args: Vec::new(),
            description: "cargo".to_owned(),
            toolchain_bin_dir: None,
        }
    }

    fn toolchain_binary(
        binary_path: PathBuf,
        toolchain_bin_dir: PathBuf,
        description: String,
    ) -> Self {
        Self {
            program: binary_path.into_os_string(),
            args: Vec::new(),
            description,
            toolchain_bin_dir: Some(toolchain_bin_dir),
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.program);
        command.args(&self.args);
        if let Some(toolchain_bin_dir) = &self.toolchain_bin_dir {
            let mut paths = vec![toolchain_bin_dir.clone()];
            if let Some(current) = env::var_os("PATH") {
                paths.extend(env::split_paths(&current));
            }
            if let Ok(joined) = env::join_paths(paths) {
                command.env("PATH", joined);
            }
        }
        command
    }
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
    target_directory: PathBuf,
    #[serde(default)]
    workspace_members: Vec<String>,
    #[serde(default)]
    workspace_default_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    id: String,
    manifest_path: PathBuf,
    targets: Vec<CargoTarget>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RustToolchainToml {
    toolchain: Option<RustToolchainTable>,
}

#[derive(Debug, Deserialize)]
struct RustToolchainTable {
    channel: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rust_toolchain_toml_channel_is_preferred() {
        let tempdir = TempDir::new().expect("tempdir should be created");
        fs::write(
            tempdir.path().join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"nightly-2025-06-23\"\n",
        )
        .expect("rust-toolchain.toml should be written");

        assert_eq!(
            requested_toolchain(tempdir.path()).as_deref(),
            Some("nightly-2025-06-23")
        );
    }

    #[test]
    fn rust_toolchain_file_uses_first_meaningful_line() {
        let tempdir = TempDir::new().expect("tempdir should be created");
        fs::write(
            tempdir.path().join("rust-toolchain"),
            "\n# comment\nstable-x86_64-unknown-linux-gnu\n",
        )
        .expect("rust-toolchain should be written");

        assert_eq!(
            requested_toolchain(tempdir.path()).as_deref(),
            Some("stable-x86_64-unknown-linux-gnu")
        );
    }

    #[test]
    fn rust_toolchain_file_accepts_toml_format_without_extension() {
        let tempdir = TempDir::new().expect("tempdir should be created");
        fs::write(
            tempdir.path().join("rust-toolchain"),
            "[toolchain]\nchannel = \"nightly-2025-06-23\"\n",
        )
        .expect("rust-toolchain should be written");

        assert_eq!(
            requested_toolchain(tempdir.path()).as_deref(),
            Some("nightly-2025-06-23")
        );
    }

    #[test]
    fn installed_toolchains_are_read_from_rustup_home() {
        let tempdir = TempDir::new().expect("tempdir should be created");
        let toolchains_dir = tempdir.path().join("toolchains");
        fs::create_dir_all(toolchains_dir.join("stable-x86_64-unknown-linux-gnu"))
            .expect("stable toolchain dir should exist");
        fs::create_dir_all(toolchains_dir.join("nightly-x86_64-unknown-linux-gnu"))
            .expect("nightly toolchain dir should exist");
        fs::write(toolchains_dir.join("README"), "ignore")
            .expect("non-directory entry should be written");

        assert_eq!(
            installed_toolchains_in_home(tempdir.path()),
            vec![
                "nightly-x86_64-unknown-linux-gnu".to_owned(),
                "stable-x86_64-unknown-linux-gnu".to_owned(),
            ]
        );
    }

    #[test]
    fn stable_channel_alias_matches_installed_host_toolchain() {
        let installed = vec![
            "nightly-x86_64-unknown-linux-gnu".to_owned(),
            "stable-x86_64-unknown-linux-gnu".to_owned(),
        ];

        assert_eq!(
            select_toolchain_name(Some("stable"), &installed).as_deref(),
            Some("stable-x86_64-unknown-linux-gnu")
        );
    }

    #[test]
    fn detects_missing_rustup_default_error() {
        let error = BuildError::CommandFailed {
            program: "cargo",
            context: "reading cargo metadata".to_owned(),
            stderr: "error: rustup could not choose a version of cargo to run, because one wasn't specified explicitly, and no default is configured.".to_owned(),
        };

        assert!(missing_rustup_default(&error));
    }

    #[test]
    fn resolves_requested_cargo_bin_case_insensitively() {
        let detected = vec!["cutty".to_owned()];

        assert_eq!(
            resolve_cargo_bin_name("CuTTY", &detected).expect("bin should resolve"),
            "cutty"
        );
    }
}
