use std::fs;
use std::process::Command;

use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::model::ForgeBrowseRecord;
use super::store::{load_submissions, local_recipe_names};
use super::workspace::CiWorkspacePaths;

pub(crate) fn handle_forge_namespace(
    app: &AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    match request.command_path.as_slice() {
        [namespace, command] if namespace == "forge" && command == "search" => {
            app.handle_forge_search(request)
        }
        [namespace, command] if namespace == "forge" && command == "browse" => {
            app.handle_forge_browse(request)
        }
        [namespace, command] if namespace == "forge" && command == "fork" => {
            app.handle_forge_fork(request)
        }
        _ => Err(CoreError::Operator("unsupported forge request".to_owned())),
    }
}

impl AppContext {
    fn handle_forge_search(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        let query = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("forge search requires one query".to_owned()))?;
        let query = query.clone();
        let query_lower = query.to_ascii_lowercase();
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let mut results = local_recipe_names(self)?
            .into_iter()
            .filter(|package| package.to_ascii_lowercase().contains(&query_lower))
            .map(|package| {
                json!({
                    "pkgname": package,
                    "source": "authoritative",
                    "packages_repo_path": workspace.packages_dir,
                })
            })
            .collect::<Vec<_>>();
        for submission in load_submissions(&workspace)? {
            for package in submission.published_packages {
                if package.pkgname.to_ascii_lowercase().contains(&query_lower) {
                    results.push(json!({
                        "pkgname": package.pkgname,
                        "source": "published",
                        "payload_path": package.payload_path,
                        "index_path": workspace.index_path,
                    }));
                }
            }
        }

        Ok(CommandReport {
            area: "forge",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("found {} forge match(es).", results.len()),
            details: Some(json!({ "query": query, "results": results })),
        })
    }

    fn handle_forge_fork(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        let repo = request.operands.first().ok_or_else(|| {
            CoreError::Operator(
                "forge fork requires `<owner/repo>` or a forge repository URL".to_owned(),
            )
        })?;
        let repo = repo.clone();
        if request.dry_run {
            return Ok(CommandReport {
                area: "forge",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!("would fork `{repo}` through the GitHub CLI (`gh repo fork`)."),
                details: Some(json!({
                    "repo": repo,
                    "forked": false,
                    "tool": "gh",
                })),
            });
        }
        let output = Command::new("gh")
            .args(["repo", "fork", &repo, "--json", "nameWithOwner,url"])
            .output()
            .map_err(|error| {
                CoreError::Operator(format!("forge fork requires `gh` on PATH: {error}"))
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::Operator(format!(
                "gh repo fork failed for `{repo}`: {}",
                stderr.trim()
            )));
        }
        let details: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(
            |_| json!({ "raw": String::from_utf8_lossy(&output.stdout).to_string() }),
        );

        Ok(CommandReport {
            area: "forge",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: false,
            summary: format!("forked `{repo}`."),
            details: Some(json!({
                "repo": repo,
                "forked": true,
                "tool": "gh",
                "result": details,
            })),
        })
    }

    fn handle_forge_browse(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        let package = request.operands.first().ok_or_else(|| {
            CoreError::Operator("forge browse requires one package name".to_owned())
        })?;
        let package = package.clone();
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let published = load_submissions(&workspace)?
            .into_iter()
            .flat_map(|submission| submission.published_packages)
            .find(|record| record.pkgname == package);
        let local_recipe_path = self
            .database
            .layout()
            .recipes_dir
            .join(&package)
            .join("pkg.lua");
        let browse = ForgeBrowseRecord {
            package: package.clone(),
            local_recipe_path: local_recipe_path
                .is_file()
                .then_some(local_recipe_path.clone()),
            packages_repo_path: workspace
                .packages_dir
                .join(&package)
                .exists()
                .then_some(workspace.packages_dir.join(&package)),
            index_path: workspace
                .index_path
                .is_file()
                .then_some(workspace.index_path.clone()),
            pkg_lua: local_recipe_path
                .is_file()
                .then(|| fs::read_to_string(local_recipe_path))
                .transpose()?,
            published,
            channel: Some("stable".to_owned()),
        };

        Ok(CommandReport {
            area: "forge",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("reported forge metadata for `{package}`."),
            details: Some(json!({ "package": browse })),
        })
    }
}
