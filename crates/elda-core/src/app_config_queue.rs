use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use crate::app::AppContext;
use crate::app_confirm::confirm_mutation;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_db::{InstallationMode, PackageFileRecord, StateLayout};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfigDiffReport {
    package: String,
    path: String,
    live_path: String,
    sidecar_path: String,
    sidecar_kind: &'static str,
    changed: bool,
    diff: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfigActionReport {
    package: String,
    path: String,
    live_path: String,
    sidecar_path: Option<String>,
    action: &'static str,
    changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ConfigPendingRecord {
    package: String,
    path: String,
    live_path: String,
    new_path: Option<String>,
    save_path: Option<String>,
    state: &'static str,
}

impl AppContext {
    pub(crate) fn handle_config_pending(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let pending = pending_config_records(&self.database)?;
        let pending_count = pending.len();

        Ok(CommandReport {
            area: "config",
            status: if pending.is_empty() { "ok" } else { "pending" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("reported {} pending configuration file(s).", pending_count),
            details: Some(json!({ "config": { "pending": pending } })),
        })
    }

    pub(crate) fn handle_config_diff(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let target = config_target(&request, "config diff")?.to_owned();
        let record = config_record_for_target(&self.database, &target)?;
        let (sidecar_kind, sidecar_path) = selected_sidecar(&record)?;
        let live_content = read_optional(&record.live_path)?;
        let sidecar_content = fs::read_to_string(&sidecar_path)?;
        let diff = content_diff(&live_content, &sidecar_content);
        let changed = live_content != sidecar_content;
        let report = ConfigDiffReport {
            package: record.package,
            path: record.path,
            live_path: record.live_path,
            sidecar_path,
            sidecar_kind,
            changed,
            diff,
        };

        Ok(CommandReport {
            area: "config",
            status: if changed { "diff" } else { "ok" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("diffed pending configuration `{target}`."),
            details: Some(json!({ "config_diff": report })),
        })
    }

    pub(crate) fn handle_config_apply(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.handle_config_action(request, true)
    }

    pub(crate) fn handle_config_keep(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.handle_config_action(request, false)
    }

    fn handle_config_action(
        &self,
        request: CommandRequest,
        apply_sidecar: bool,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let command = if apply_sidecar {
            "config apply"
        } else {
            "config keep"
        };
        let target = config_target(&request, command)?.to_owned();
        let record = config_record_for_target(&self.database, &target)?;
        let verb = if apply_sidecar { "apply" } else { "keep" };
        confirm_mutation(
            &request,
            &format!("{verb} pending configuration for `{target}`?"),
        )?;
        let report = if apply_sidecar {
            apply_config_sidecar(&record, request.dry_run)?
        } else {
            keep_config_live(&record, request.dry_run)?
        };

        Ok(CommandReport {
            area: "config",
            status: if request.dry_run { "planned" } else { "ok" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("{} pending configuration `{target}`.", report.action),
            details: Some(json!({ "config_action": report })),
        })
    }
}

pub(crate) fn pending_config_count(app: &AppContext) -> Result<usize, CoreError> {
    Ok(pending_config_records(&app.database)?.len())
}

fn pending_config_records(
    database: &elda_db::Database,
) -> Result<Vec<ConfigPendingRecord>, CoreError> {
    let mut records = Vec::new();
    for package in database.list_installed_packages()? {
        for file in database.package_files(&package.pkgname)? {
            if let Some(record) = pending_record(database.layout(), &file)? {
                records.push(record);
            }
        }
    }
    records.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(records)
}

fn pending_record(
    layout: &StateLayout,
    file: &PackageFileRecord,
) -> Result<Option<ConfigPendingRecord>, CoreError> {
    if !file.is_conffile || file.path_kind != "file" {
        return Ok(None);
    }

    let live_path = live_config_path(layout, &file.path)?;
    let new_path = sidecar_if_exists(&live_path, ".eldanew");
    let save_path = sidecar_if_exists(&live_path, ".eldasave");
    if new_path.is_none() && save_path.is_none() {
        return Ok(None);
    }

    Ok(Some(ConfigPendingRecord {
        package: file.pkgname.clone(),
        path: file.path.clone(),
        live_path: live_path.display().to_string(),
        new_path: new_path.map(|path| path.display().to_string()),
        save_path: save_path.map(|path| path.display().to_string()),
        state: if sidecar_if_exists(&live_path, ".eldanew").is_some() {
            "merge-new"
        } else {
            "saved-on-remove"
        },
    }))
}

fn live_config_path(layout: &StateLayout, manifest_path: &str) -> Result<PathBuf, CoreError> {
    if layout.mode == InstallationMode::System {
        return Ok(layout.root_dir.join(strip_leading_slash(manifest_path)?));
    }

    let prefix_string = layout.prefix.to_string_lossy();
    let prefix_root = strip_leading_slash(&prefix_string)?;
    if let Some(suffix) = manifest_path.strip_prefix("/etc/") {
        return Ok(layout.root_dir.join(prefix_root).join("etc").join(suffix));
    }

    Ok(layout
        .root_dir
        .join(prefix_root)
        .join(strip_leading_slash(manifest_path)?))
}

fn sidecar_if_exists(path: &std::path::Path, extension: &str) -> Option<PathBuf> {
    let sidecar = PathBuf::from(format!("{}{}", path.display(), extension));
    if sidecar.exists() || sidecar.is_symlink() {
        Some(sidecar)
    } else {
        None
    }
}

fn strip_leading_slash(path: &str) -> Result<&str, CoreError> {
    path.strip_prefix('/').ok_or_else(|| {
        CoreError::Operator(format!(
            "managed configuration path `{path}` is not absolute"
        ))
    })
}

fn config_target<'a>(request: &'a CommandRequest, command: &str) -> Result<&'a str, CoreError> {
    request
        .operands
        .first()
        .map(String::as_str)
        .ok_or_else(|| CoreError::Operator(format!("{command} requires a package or path")))
}

fn config_record_for_target(
    database: &elda_db::Database,
    target: &str,
) -> Result<ConfigPendingRecord, CoreError> {
    pending_config_records(database)?
        .into_iter()
        .find(|record| record_matches_target(record, target))
        .ok_or_else(|| {
            CoreError::Operator(format!("no pending configuration file matches `{target}`"))
        })
}

fn record_matches_target(record: &ConfigPendingRecord, target: &str) -> bool {
    record.package == target
        || record.path == target
        || record.live_path == target
        || record.new_path.as_deref() == Some(target)
        || record.save_path.as_deref() == Some(target)
}

fn selected_sidecar(record: &ConfigPendingRecord) -> Result<(&'static str, String), CoreError> {
    if let Some(path) = &record.new_path {
        return Ok(("eldanew", path.clone()));
    }
    if let Some(path) = &record.save_path {
        return Ok(("eldasave", path.clone()));
    }
    Err(CoreError::Operator(format!(
        "configuration `{}` has no pending sidecar",
        record.path
    )))
}

fn read_optional(path: &str) -> Result<String, CoreError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(CoreError::Io(error)),
    }
}

fn content_diff(current: &str, candidate: &str) -> Vec<String> {
    let current_lines = current.lines().collect::<Vec<_>>();
    let candidate_lines = candidate.lines().collect::<Vec<_>>();
    let max_len = current_lines.len().max(candidate_lines.len());
    let mut diff = Vec::new();
    for index in 0..max_len {
        let current = current_lines.get(index).copied();
        let candidate = candidate_lines.get(index).copied();
        if current == candidate {
            continue;
        }
        let line_no = index + 1;
        if let Some(line) = current {
            diff.push(format!("-{line_no}: {line}"));
        }
        if let Some(line) = candidate {
            diff.push(format!("+{line_no}: {line}"));
        }
        if diff.len() >= 80 {
            diff.push("... diff truncated after 80 changed lines".to_owned());
            break;
        }
    }
    if diff.is_empty() {
        diff.push("no content changes".to_owned());
    }
    diff
}

fn apply_config_sidecar(
    record: &ConfigPendingRecord,
    dry_run: bool,
) -> Result<ConfigActionReport, CoreError> {
    let (sidecar_kind, sidecar_path) = selected_sidecar(record)?;
    if !dry_run {
        if let Some(parent) = Path::new(&record.live_path).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&sidecar_path, &record.live_path)?;
        fs::remove_file(&sidecar_path)?;
    }

    Ok(ConfigActionReport {
        package: record.package.clone(),
        path: record.path.clone(),
        live_path: record.live_path.clone(),
        sidecar_path: Some(sidecar_path),
        action: if sidecar_kind == "eldanew" {
            "applied"
        } else {
            "restored"
        },
        changed: !dry_run,
    })
}

fn keep_config_live(
    record: &ConfigPendingRecord,
    dry_run: bool,
) -> Result<ConfigActionReport, CoreError> {
    let (_, sidecar_path) = selected_sidecar(record)?;
    if !dry_run {
        fs::remove_file(&sidecar_path)?;
    }

    Ok(ConfigActionReport {
        package: record.package.clone(),
        path: record.path.clone(),
        live_path: record.live_path.clone(),
        sidecar_path: Some(sidecar_path),
        action: "kept-live",
        changed: !dry_run,
    })
}
