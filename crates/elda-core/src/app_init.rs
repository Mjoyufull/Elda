use std::fs;

use serde_json::json;

use crate::app::AppContext;
use crate::config::Config;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_init(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        let layout = self.database.layout();
        let paths = [
            ("config_dir", layout.config_dir.clone()),
            ("recipes_dir", layout.recipes_dir.clone()),
            ("remotes_dir", layout.remotes_dir.clone()),
            ("caches_dir", layout.caches_dir.clone()),
            ("extensions_dir", layout.extensions_dir.clone()),
            ("state_dir", layout.state_dir.clone()),
            ("manifests_dir", layout.manifests_dir.clone()),
            ("cache_pkg_dir", layout.cache_pkg_dir.clone()),
            ("cache_src_dir", layout.cache_src_dir.clone()),
            ("tmp_dir", layout.tmp_dir.clone()),
        ];

        let mut created = Vec::new();
        let mut existing = Vec::new();
        for (label, path) in paths {
            if path.exists() {
                existing.push(json!({ "label": label, "path": path.display().to_string() }));
            } else if request.dry_run {
                created.push(
                    json!({ "label": label, "path": path.display().to_string(), "planned": true }),
                );
            } else {
                fs::create_dir_all(&path)?;
                created.push(
                    json!({ "label": label, "path": path.display().to_string(), "created": true }),
                );
            }
        }

        let config_path = layout.config_dir.join("config.toml");
        let config_created = if config_path.exists() {
            false
        } else if request.dry_run {
            true
        } else {
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            Config::write_default(&layout.root_dir)?;
            true
        };

        let bootstrap = if request.dry_run {
            None
        } else {
            Some(self.database.bootstrap()?)
        };

        Ok(CommandReport {
            area: "init",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if request.dry_run {
                "planned Elda bootstrap directories and default config.".to_owned()
            } else {
                format!(
                    "initialized Elda layout ({} new directories, config created: {config_created}).",
                    created.len()
                )
            },
            details: Some(json!({
                "created": created,
                "existing": existing,
                "config_path": config_path.display().to_string(),
                "config_created": config_created,
                "database": bootstrap.map(|report| json!({
                    "schema_version": report.schema_version,
                    "created_database": report.created_database,
                })),
            })),
        })
    }
}
