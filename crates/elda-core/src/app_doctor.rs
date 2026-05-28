use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::{load_system_backend_status, pending_triggers};
use elda_repo::list_remotes;

impl AppContext {
    pub(crate) fn handle_doctor(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let layout = self.database.layout();
        let health = self.database.health_report()?;
        let installed = self.database.list_installed_packages()?;
        let remotes = list_remotes(&layout.remotes_dir)?;
        let pending_triggers = pending_triggers(layout)?;
        let backend = load_system_backend_status(layout)?;
        let paths = vec![
            path_check("config", &layout.config_dir),
            path_check("config.toml", &layout.config_dir.join("config.toml")),
            path_check("recipes", &layout.recipes_dir),
            path_check("remotes", &layout.remotes_dir),
            path_check("caches", &layout.caches_dir),
            path_check("extensions", &layout.extensions_dir),
            path_check("database", &layout.db_path),
            path_check("manifests", &layout.manifests_dir),
            path_check("state", &layout.state_dir),
            path_check("package-cache", &layout.cache_pkg_dir),
            path_check("source-cache", &layout.cache_src_dir),
            path_check("tmp", &layout.tmp_dir),
        ];
        let missing_required_paths = paths
            .iter()
            .filter(|path| {
                path.get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                    && !path.get("exists").and_then(Value::as_bool).unwrap_or(false)
            })
            .count();
        let critical_pending_triggers = pending_triggers
            .iter()
            .filter(|record| record.critical)
            .count();
        let mut blockers = health.issues.clone();
        if missing_required_paths > 0 {
            blockers.push(format!(
                "{missing_required_paths} required Elda path(s) are missing after bootstrap"
            ));
        }
        if critical_pending_triggers > 0 {
            blockers.push(format!(
                "{critical_pending_triggers} critical pending system trigger(s) require repair"
            ));
        }
        let advisories = doctor_advisories(remotes.len(), installed.len(), pending_triggers.len());
        let status = if blockers.is_empty() { "ok" } else { "issues" };

        Ok(CommandReport {
            area: "doctor",
            status,
            exit_status: if blockers.is_empty() {
                ExitStatus::Success
            } else {
                ExitStatus::OperatorFailure
            },
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "doctor checked {} path(s), {} remote(s), and {} installed package(s).",
                paths.len(),
                remotes.len(),
                installed.len()
            ),
            details: Some(json!({
                "mode": format!("{:?}", layout.mode).to_ascii_lowercase(),
                "root": layout.root_dir,
                "prefix": layout.prefix,
                "paths": paths,
                "counts": {
                    "installed_packages": installed.len(),
                    "configured_remotes": remotes.len(),
                    "local_recipes": count_entries(&layout.recipes_dir),
                    "configured_caches": count_entries(&layout.caches_dir),
                    "extensions": count_entries(&layout.extensions_dir),
                    "pending_triggers": pending_triggers.len(),
                },
                "health": health,
                "backend": backend,
                "remotes": remotes,
                "pending_triggers": pending_triggers,
                "advisories": advisories,
                "blockers": blockers,
                "release_readiness": {
                    "unsupported_commands_fail_closed": true,
                    "check_and_verify_fail_on_issues": true,
                    "dry_run_preflight_visible": true,
                },
            })),
        })
    }
}

fn path_check(label: &str, path: &Path) -> Value {
    json!({
        "label": label,
        "path": path,
        "exists": path.exists(),
        "required": true,
    })
}

fn count_entries(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0)
}

fn doctor_advisories(remotes: usize, installed: usize, pending_triggers: usize) -> Vec<String> {
    let mut advisories = Vec::new();
    if remotes == 0 {
        advisories.push(
            "no remotes are configured; add one with `elda rmt add` before normal synced installs"
                .to_owned(),
        );
    }
    if installed == 0 {
        advisories.push("no packages are recorded in this root yet".to_owned());
    }
    if pending_triggers > 0 {
        advisories.push("pending system triggers exist; run `elda fix-triggers` after reviewing `elda trigger ls`".to_owned());
    }
    advisories
}
