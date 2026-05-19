use serde_json::json;

use crate::{CommandReport, CommandRequest, CoreError, ExitStatus, OutputMode};

pub fn report_runtime_failure(error: &CoreError, request: &CommandRequest) -> CommandReport {
    let exit_status = classify_error(error);
    let kind = failure_kind(exit_status);
    let area = failure_area(request);
    let blocked = error.to_string();
    let causes = error_causes(error, &blocked);

    CommandReport {
        area,
        status: "blocked",
        exit_status,
        command_path: request.command_path.clone(),
        operands: request.operands.clone(),
        output_mode: request.output_mode,
        dry_run: request.dry_run,
        summary: format!("blocked {kind}: {blocked}"),
        details: Some(json!({
            "blocked": blocked,
            "kind": kind,
            "command_path": request.command_path,
            "operands": request.operands,
            "dry_run": request.dry_run,
            "system_mode": request.system_mode,
            "offline": request.offline,
            "causes": causes,
            "next_action": next_action(error),
        })),
    }
}

pub fn report_frontend_failure(blocked: String, causes: Vec<String>) -> CommandReport {
    CommandReport {
        area: "cli",
        status: "blocked",
        exit_status: ExitStatus::OperatorFailure,
        command_path: Vec::new(),
        operands: Vec::new(),
        output_mode: OutputMode::Human,
        dry_run: false,
        summary: format!("blocked operator failure: {blocked}"),
        details: Some(json!({
            "blocked": blocked,
            "kind": "operator failure",
            "command_path": [],
            "operands": [],
            "dry_run": false,
            "system_mode": false,
            "offline": false,
            "causes": causes,
            "next_action": "fix the CLI invocation or environment and retry",
        })),
    }
}

fn classify_error(error: &CoreError) -> ExitStatus {
    match error {
        CoreError::Repo(elda_repo::RepoError::Trust(_)) => ExitStatus::TrustFailure,
        CoreError::Repo(elda_repo::RepoError::SnapshotMissing)
        | CoreError::Recipe(_)
        | CoreError::Build(elda_build::BuildError::Invalid(_))
        | CoreError::Build(elda_build::BuildError::Unsupported(_))
        | CoreError::Install(elda_install::InstallError::Unsupported(_))
        | CoreError::Install(elda_install::InstallError::AlreadyInstalled(_))
        | CoreError::Install(elda_install::InstallError::NotInstalled(_))
        | CoreError::Install(elda_install::InstallError::PathConflict { .. })
        | CoreError::Install(elda_install::InstallError::UnmanagedPathCollision(_)) => {
            ExitStatus::ResolutionFailure
        }
        CoreError::PrivilegeRequired(_) => ExitStatus::OperatorFailure,
        CoreError::Io(_)
        | CoreError::Toml(_)
        | CoreError::Json(_)
        | CoreError::Build(_)
        | CoreError::Install(_)
        | CoreError::Repo(_)
        | CoreError::Db(_)
        | CoreError::Operator(_) => ExitStatus::OperatorFailure,
    }
}

fn failure_kind(exit_status: ExitStatus) -> &'static str {
    match exit_status {
        ExitStatus::Success => "success",
        ExitStatus::OperatorFailure => "operator failure",
        ExitStatus::ResolutionFailure => "resolution failure",
        ExitStatus::TrustFailure => "trust failure",
    }
}

fn failure_area(request: &CommandRequest) -> &'static str {
    match request.command_path.first().map(String::as_str) {
        Some("i" | "ig" | "ib") => "install",
        Some("u") => "upgrade",
        Some("sync" | "rmt") => "repo",
        Some("rc") => "recipe",
        Some("ci") => "ci",
        Some("pf") => "profile",
        Some("state") => "state",
        Some("cache") => "cache",
        Some("daemon") => "daemon",
        Some("qa") => "qa",
        _ => "command",
    }
}

fn error_causes(error: &CoreError, root_message: &str) -> Vec<String> {
    std::error::Error::source(error)
        .into_iter()
        .map(ToString::to_string)
        .filter(|cause| cause != root_message)
        .collect()
}

fn next_action(error: &CoreError) -> &'static str {
    match error {
        CoreError::Repo(elda_repo::RepoError::Trust(_)) => {
            "inspect the remote trust record; use --accept-rotated-key only for verified rotations"
        }
        CoreError::Repo(elda_repo::RepoError::SnapshotMissing) => {
            "run `elda sync` for the configured remotes and retry"
        }
        CoreError::Install(elda_install::InstallError::PendingRecovery(_)) => {
            "run `elda recover` before the next mutating command"
        }
        CoreError::Install(elda_install::InstallError::PathConflict { .. })
        | CoreError::Install(elda_install::InstallError::UnmanagedPathCollision(_)) => {
            "inspect the conflicting path owner, then remove, adopt, or choose another package"
        }
        CoreError::Build(elda_build::BuildError::Unsupported(_)) => {
            "use a supported source shape or add the missing parser/build-system support"
        }
        CoreError::Build(elda_build::BuildError::CommandFailed { .. }) => {
            "inspect the build stderr and session log, then retry after fixing the build input"
        }
        CoreError::Recipe(_) => "fix the recipe metadata and retry",
        CoreError::Db(_) => "inspect the Elda state database and retry after recovery",
        CoreError::PrivilegeRequired(_) => "rerun with the configured privilege provider",
        _ => "fix the reported input or environment and retry",
    }
}

pub fn process_exit_code(status: ExitStatus) -> i32 {
    match status {
        ExitStatus::Success => 0,
        ExitStatus::OperatorFailure => 1,
        ExitStatus::ResolutionFailure => 2,
        ExitStatus::TrustFailure => 3,
    }
}
