use std::path::Path;

use serde_json::json;

use crate::app::AppContext;
use crate::app_recipe_metadata::metadata_add_json;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_metadata_add(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let bootstrap = self.database.bootstrap()?;
        let parsed = self.parse_install_request(&request)?;
        if parsed.targets.is_empty() {
            return Err(CoreError::Operator(
                "metadata add requires at least one link or local source".to_owned(),
            ));
        }

        let mut added = Vec::with_capacity(parsed.targets.len());
        let mut snapshots = Vec::new();

        for target in &parsed.targets {
            match self.resolve_any_install_target(target, &parsed)? {
                crate::app_install::ResolutionReport::Single(resolved) => {
                    let recipe_dir = resolved
                        .generated_recipe_dir
                        .clone()
                        .or_else(|| resolved.recipe.path.parent().map(Path::to_path_buf))
                        .ok_or_else(|| {
                            CoreError::Operator(format!(
                                "could not determine metadata directory for `{target}`"
                            ))
                        })?;
                    added.push(metadata_add_json(target, &resolved, &recipe_dir));
                }
                crate::app_install::ResolutionReport::Bulk(snapshot) => {
                    snapshots.push(snapshot);
                }
            }
        }

        self.review_bulk_snapshot_if_needed(&request, &snapshots)?;

        Ok(CommandReport {
            area: "metadata",
            status: if !snapshots.is_empty() {
                "snapshot_detected"
            } else if request.dry_run {
                "planned"
            } else {
                "ok"
            },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if snapshots.is_empty() {
                format!("prepared metadata for {} source target(s).", added.len())
            } else {
                format!(
                    "prepared metadata for {} targets and detected {} bulk snapshots.",
                    added.len(),
                    snapshots.len()
                )
            },
            details: Some(json!({
                "schema_version": bootstrap.schema_version,
                "created_database": bootstrap.created_database,
                "layout": self.database.layout(),
                "metadata_add": {
                    "link_option_mode": self.config.metadata.link_option_mode,
                    "targets": added,
                },
                "bulk_snapshots": snapshots,
            })),
        })
    }
}
