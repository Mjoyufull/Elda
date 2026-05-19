use serde_json::json;

use crate::app::AppContext;
use crate::app_ci::{CiWorkspacePaths, publish_workspace};
use crate::app_host::{
    discover_package_names, git_tree_commit, resolve_recipe_tree, sync_tree_packages_to_recipes,
};
use crate::app_publish::plan::{parse_flag_value, parse_publish_scope, publish_plan_for_targets};
use crate::error::CoreError;
use crate::host_config::load_host_profile;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_publish_run(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        if request.dry_run {
            return self.handle_publish_plan(request);
        }

        self.database.bootstrap()?;
        let layout = self.database.layout();
        let (mut targets, channel) = parse_publish_scope(&request)?;
        let tree_path = parse_flag_value(&request, "--tree");
        let repo_commit = if let Some(tree_path) = &tree_path {
            let tree = resolve_recipe_tree(std::path::Path::new(tree_path), "packages")?;
            targets = discover_package_names(&tree, &targets)?;
            sync_tree_packages_to_recipes(&tree, &layout.recipes_dir, &targets)?;
            git_tree_commit(&tree.root)?
        } else {
            None
        };

        if targets.is_empty() {
            return Err(CoreError::Operator(
                "`publish run` requires package names or `--tree <path>`".to_owned(),
            ));
        }

        let profile = load_host_profile(
            &layout.root_dir,
            parse_flag_value(&request, "--profile").as_deref(),
        )
        .ok();
        let workspace = CiWorkspacePaths::for_channel(layout, &channel);
        workspace.ensure_exists()?;
        if let Some(profile) = &profile {
            if let Some(key_path) = profile.signing_key_path() {
                if key_path.is_file() {
                    std::fs::copy(&key_path, &workspace.signing_key_path)?;
                }
            }
        }

        let plan = publish_plan_for_targets(self, &targets)?;
        let submission_id = format!("host-publish-{channel}");
        let published = publish_workspace(self, &workspace, &plan, &submission_id, &channel)?;

        Ok(CommandReport {
            area: "publish",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "published {} package(s) to {} for channel `{channel}`.",
                published.packages.len(),
                workspace.published_dir.display()
            ),
            details: Some(json!({
                "channel": channel,
                "repo_commit": repo_commit.or(published.repo_commit),
                "published_dir": workspace.published_dir,
                "index_path": workspace.index_path,
                "packages": published.packages.iter().map(|pkg| json!({
                    "pkgname": pkg.pkgname,
                    "pkgver": pkg.pkgver,
                    "arch": pkg.arch,
                })).collect::<Vec<_>>(),
            })),
        })
    }
}
