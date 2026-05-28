use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use elda_db::{InstallationMode, StateLayout};

use crate::MutationPolicy;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub phase: String,
    pub tool: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotPhase {
    PreActivation,
    PostActivation,
}

impl SnapshotPhase {
    const fn as_str(self) -> &'static str {
        match self {
            Self::PreActivation => "pre-activation",
            Self::PostActivation => "post-activation",
        }
    }

    fn description(self, state_id: &str) -> String {
        format!("elda {} {state_id}", self.as_str())
    }
}

pub(crate) fn pre_activation_snapshot(
    layout: &StateLayout,
    state_id: &str,
    policy: &MutationPolicy,
) -> Option<SnapshotRecord> {
    request_snapshot(layout, state_id, policy, SnapshotPhase::PreActivation)
}

pub(crate) fn post_activation_snapshot(
    layout: &StateLayout,
    state_id: &str,
    policy: &MutationPolicy,
) -> Option<SnapshotRecord> {
    request_snapshot(layout, state_id, policy, SnapshotPhase::PostActivation)
}

fn request_snapshot(
    layout: &StateLayout,
    state_id: &str,
    policy: &MutationPolicy,
    phase: SnapshotPhase,
) -> Option<SnapshotRecord> {
    if layout.mode != InstallationMode::System {
        return None;
    }

    let tool = normalized_tool(policy)?;
    let command = match snapshot_command(layout, &tool, state_id, phase) {
        Ok(command) => command,
        Err(error) => {
            return Some(SnapshotRecord {
                phase: phase.as_str().to_owned(),
                tool,
                status: "failed".to_owned(),
                snapshot_id: None,
                error: Some(error),
            });
        }
    };

    Some(run_snapshot_command(&tool, phase, command))
}

fn normalized_tool(policy: &MutationPolicy) -> Option<String> {
    policy
        .snapshot_tool
        .as_deref()
        .map(str::trim)
        .filter(|tool| !tool.is_empty() && *tool != "none")
        .map(ToOwned::to_owned)
}

struct SnapshotCommand {
    program: PathBuf,
    args: Vec<String>,
    snapshot_id_hint: Option<String>,
}

fn snapshot_command(
    layout: &StateLayout,
    configured_tool: &str,
    state_id: &str,
    phase: SnapshotPhase,
) -> Result<SnapshotCommand, String> {
    let program = PathBuf::from(configured_tool);
    let tool_name = program
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(configured_tool);

    match tool_name {
        "snapper" => Ok(SnapshotCommand {
            program,
            args: vec![
                "create".to_owned(),
                "--print-number".to_owned(),
                "--description".to_owned(),
                phase.description(state_id),
            ],
            snapshot_id_hint: None,
        }),
        "btrfs" => {
            let snapshot_path = layout
                .data_dir
                .join("snapshots")
                .join(format!("{state_id}-{}", phase.as_str()));
            if let Some(parent) = snapshot_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create snapshot directory `{}`: {error}",
                        parent.display()
                    )
                })?;
            }
            Ok(SnapshotCommand {
                program,
                args: vec![
                    "subvolume".to_owned(),
                    "snapshot".to_owned(),
                    "-r".to_owned(),
                    layout.root_dir.display().to_string(),
                    snapshot_path.display().to_string(),
                ],
                snapshot_id_hint: Some(snapshot_path.display().to_string()),
            })
        }
        other => Err(format!(
            "snapshot tool `{other}` is not supported by the current backend"
        )),
    }
}

fn run_snapshot_command(
    tool: &str,
    phase: SnapshotPhase,
    command: SnapshotCommand,
) -> SnapshotRecord {
    match Command::new(&command.program).args(&command.args).output() {
        Ok(output) if output.status.success() => SnapshotRecord {
            phase: phase.as_str().to_owned(),
            tool: tool.to_owned(),
            status: "captured".to_owned(),
            snapshot_id: first_non_empty_line(&output.stdout).or(command.snapshot_id_hint),
            error: None,
        },
        Ok(output) => SnapshotRecord {
            phase: phase.as_str().to_owned(),
            tool: tool.to_owned(),
            status: "failed".to_owned(),
            snapshot_id: None,
            error: Some(command_failure_message(&command.program, &output)),
        },
        Err(error) => SnapshotRecord {
            phase: phase.as_str().to_owned(),
            tool: tool.to_owned(),
            status: "failed".to_owned(),
            snapshot_id: None,
            error: Some(format!(
                "failed to execute `{}`: {error}",
                command.program.display()
            )),
        },
    }
}

fn command_failure_message(program: &std::path::Path, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        format!("exit status {}", output.status)
    };

    format!("snapshot command `{}` failed: {detail}", program.display())
}

fn first_non_empty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}
