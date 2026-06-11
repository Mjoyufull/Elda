use std::env;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus, OutputMode};

const ELDA_REPOSITORY: &str = "https://github.com/Mjoyufull/Elda";

impl AppContext {
    pub(crate) fn handle_self_update(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        if request.offline {
            return Err(CoreError::Operator(
                "self-update requires network access; remove `--offline` and retry".to_owned(),
            ));
        }

        let branch = update_branch(&request.command_path)?;
        let executable = env::current_exe()?;
        if request.dry_run {
            return Ok(self_update_report(request, branch, &executable, None, None));
        }

        let workspace = tempfile::tempdir()?;
        let source_dir = workspace.path().join("Elda");
        clone_branch(branch, &source_dir, request.output_mode)?;
        build_cli(&source_dir, request.output_mode)?;

        let built_binary = source_dir.join("target/release/elda");
        verify_built_binary(&built_binary)?;
        let commit = git_commit(&source_dir)?;
        let built_version = binary_version(&built_binary)?;
        replace_executable(&built_binary, &executable)?;

        Ok(self_update_report(
            request,
            branch,
            &executable,
            Some(commit),
            Some(built_version),
        ))
    }
}

fn update_branch(command_path: &[String]) -> Result<&'static str, CoreError> {
    match command_path {
        [command] if command == "su" => Ok("main"),
        [command] if command == "dsu" => Ok("dev"),
        _ => Err(CoreError::Operator(
            "invalid self-update command path".to_owned(),
        )),
    }
}

fn clone_branch(branch: &str, source_dir: &Path, output_mode: OutputMode) -> Result<(), CoreError> {
    let mut command = Command::new("git");
    command.args([
        "clone",
        "--depth",
        "1",
        "--single-branch",
        "--branch",
        branch,
        ELDA_REPOSITORY,
    ]);
    command.arg(source_dir);
    configure_operator_environment(&mut command);
    run_command(&mut command, "git clone", output_mode)
}

fn build_cli(source_dir: &Path, output_mode: OutputMode) -> Result<(), CoreError> {
    let mut command = Command::new(operator_cargo());
    command
        .current_dir(source_dir)
        .args(["build", "--release", "--locked", "-p", "elda-cli"]);
    configure_operator_environment(&mut command);
    run_command(&mut command, "cargo build", output_mode)
}

fn run_command(
    command: &mut Command,
    label: &str,
    output_mode: OutputMode,
) -> Result<(), CoreError> {
    if output_mode == OutputMode::Human {
        let status = command
            .status()
            .map_err(|error| command_error(label, error))?;
        if status.success() {
            return Ok(());
        }
        return Err(CoreError::Operator(format!(
            "{label} failed with status {status}"
        )));
    }

    let output = command
        .output()
        .map_err(|error| command_error(label, error))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::Operator(format!(
            "{label} failed with status {}: {}",
            output.status,
            output_message(&output)
        )))
    }
}

fn command_error(label: &str, error: io::Error) -> CoreError {
    CoreError::Operator(format!("failed to start {label}: {error}"))
}

fn output_message(output: &Output) -> String {
    let bytes = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    let message = String::from_utf8_lossy(bytes);
    let trimmed = message.trim();
    if trimmed.is_empty() {
        "no command output".to_owned()
    } else {
        trimmed.chars().take(4096).collect()
    }
}

fn configure_operator_environment(command: &mut Command) {
    let Some(home) = env::var_os("ELDA_OPERATOR_HOME").map(PathBuf::from) else {
        return;
    };

    let cargo_home = home.join(".cargo");
    let rustup_home = home.join(".rustup");
    command.env("HOME", &home);
    command.env("CARGO_HOME", &cargo_home);
    command.env("RUSTUP_HOME", rustup_home);
    if let Some(path) = env::var_os("PATH") {
        let mut paths = vec![cargo_home.join("bin")];
        paths.extend(env::split_paths(&path));
        if let Ok(joined) = env::join_paths(paths) {
            command.env("PATH", joined);
        }
    }
}

fn operator_cargo() -> PathBuf {
    env::var_os("ELDA_OPERATOR_HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".cargo/bin/cargo"))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from("cargo"))
}

fn verify_built_binary(path: &Path) -> Result<(), CoreError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(CoreError::Operator(format!(
            "self-update build did not produce `{}`",
            path.display()
        )))
    }
}

fn git_commit(source_dir: &Path) -> Result<String, CoreError> {
    let output = Command::new("git")
        .current_dir(source_dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|error| command_error("git rev-parse", error))?;
    checked_stdout("git rev-parse", output)
}

fn binary_version(path: &Path) -> Result<String, CoreError> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| command_error("built Elda version check", error))?;
    checked_stdout("built Elda version check", output)
}

fn checked_stdout(label: &str, output: Output) -> Result<String, CoreError> {
    if !output.status.success() {
        return Err(CoreError::Operator(format!(
            "{label} failed with status {}: {}",
            output.status,
            output_message(&output)
        )));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() {
        Err(CoreError::Operator(format!("{label} returned no output")))
    } else {
        Ok(value)
    }
}

fn replace_executable(source: &Path, destination: &Path) -> Result<(), CoreError> {
    let destination = destination.canonicalize()?;
    let parent = destination.parent().ok_or_else(|| {
        CoreError::Operator("current Elda executable has no parent directory".to_owned())
    })?;
    let metadata = fs::metadata(&destination)?;
    let staging = parent.join(format!(".elda-self-update-{}", std::process::id()));

    let result = stage_replacement(source, &staging, &metadata)
        .and_then(|()| fs::rename(&staging, &destination).map_err(CoreError::from));
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result?;

    File::open(parent)?.sync_all()?;
    Ok(())
}

fn stage_replacement(
    source: &Path,
    staging: &Path,
    destination_metadata: &fs::Metadata,
) -> Result<(), CoreError> {
    let mut source_file = File::open(source)?;
    let mut staging_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(staging)?;
    io::copy(&mut source_file, &mut staging_file)?;
    staging_file.set_permissions(fs::Permissions::from_mode(
        destination_metadata.permissions().mode(),
    ))?;

    let staging_metadata = staging_file.metadata()?;
    if staging_metadata.uid() != destination_metadata.uid()
        || staging_metadata.gid() != destination_metadata.gid()
    {
        std::os::unix::fs::chown(
            staging,
            Some(destination_metadata.uid()),
            Some(destination_metadata.gid()),
        )?;
    }
    staging_file.sync_all()?;
    Ok(())
}

fn self_update_report(
    request: CommandRequest,
    branch: &str,
    executable: &Path,
    commit: Option<String>,
    version: Option<String>,
) -> CommandReport {
    let dry_run = request.dry_run;
    CommandReport {
        area: "self-update",
        status: if dry_run { "planned" } else { "ok" },
        exit_status: ExitStatus::Success,
        command_path: request.command_path,
        operands: request.operands,
        output_mode: request.output_mode,
        dry_run,
        summary: if dry_run {
            format!("would update Elda from `{branch}`")
        } else {
            format!("updated Elda from `{branch}`")
        },
        details: Some(json!({
            "repository": ELDA_REPOSITORY,
            "branch": branch,
            "executable": executable,
            "commit": commit,
            "version": version,
        })),
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::{replace_executable, update_branch};

    #[test]
    fn self_update_commands_select_expected_branches() {
        assert_eq!(
            update_branch(&["su".to_owned()]).expect("main branch"),
            "main"
        );
        assert_eq!(
            update_branch(&["dsu".to_owned()]).expect("dev branch"),
            "dev"
        );
    }

    #[test]
    fn executable_replacement_preserves_destination_mode() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let source = tempdir.path().join("new-elda");
        let destination = tempdir.path().join("elda");
        std::fs::write(&source, b"new binary").expect("source should be written");
        std::fs::write(&destination, b"old binary").expect("destination should be written");
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o751))
            .expect("destination mode should be set");

        replace_executable(&source, &destination).expect("replacement should succeed");

        assert_eq!(
            std::fs::read(&destination).expect("replacement should be readable"),
            b"new binary"
        );
        let mode = std::fs::metadata(&destination)
            .expect("replacement metadata should be readable")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o751);
    }
}
