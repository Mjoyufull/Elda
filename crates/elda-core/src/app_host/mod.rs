mod scan;
pub(crate) mod tree;

pub(crate) use tree::{
    discover_package_names, git_tree_commit, resolve_recipe_tree, sync_tree_packages_to_recipes,
};

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::json;

use crate::app::AppContext;
use crate::app_ci::CiWorkspacePaths;
use crate::app_host::scan::{scan_tree_json, scan_tree_packages};
use crate::app_host::tree::{RecipeTree, git_changed_packages_since};
use crate::app_publish::publish_plan_for_targets;
use crate::error::CoreError;
use crate::host_config::{ResolvedHostProfile, load_host_profile};
use crate::{CommandReport, CommandRequest, ExitStatus};

pub(crate) fn handle_host_namespace(
    app: &AppContext,
    request: CommandRequest,
) -> Result<CommandReport, CoreError> {
    match request.command_path.as_slice() {
        [namespace, command] if namespace == "host" && command == "scan-tree" => {
            app.handle_host_scan_tree(request)
        }
        [namespace, command] if namespace == "host" && command == "test-tree" => {
            app.handle_host_test_tree(request)
        }
        [namespace, command] if namespace == "host" && command == "diff-tree" => {
            app.handle_host_diff_tree(request)
        }
        [namespace, command] if namespace == "host" && command == "client-bundle" => {
            app.handle_host_client_bundle(request)
        }
        [namespace, command] if namespace == "host" && command == "status" => {
            app.handle_host_status(request)
        }
        [namespace, command] if namespace == "host" && command == "doctor" => {
            app.handle_host_doctor(request)
        }
        [namespace, command] if namespace == "host" && command == "init-ci" => {
            app.handle_host_init_ci(request)
        }
        [namespace, command] if namespace == "host" && command == "link" => {
            app.handle_host_link(request)
        }
        [namespace, command] if namespace == "host" && command == "push-recipes" => {
            app.handle_host_push_recipes(request)
        }
        [namespace, command] if namespace == "host" && command == "print-cache-config" => {
            app.handle_host_print_cache_config(request)
        }
        _ => Err(CoreError::Operator("unsupported host request".to_owned())),
    }
}

impl AppContext {
    fn resolve_host_tree(
        &self,
        request: &CommandRequest,
        profile: Option<&ResolvedHostProfile>,
    ) -> Result<(RecipeTree, ResolvedHostProfile), CoreError> {
        let profile = match profile {
            Some(value) => value.clone(),
            None => load_host_profile(
                &self.database.layout().root_dir,
                host_profile_name(request).as_deref(),
            )
            .unwrap_or_else(|_| ResolvedHostProfile {
                name: "inline".to_owned(),
                path: self.database.layout().config_dir.join("host.d"),
                section: Default::default(),
            }),
        };
        let packages_subdir = profile.packages_subdir();
        let tree_path = host_tree_path(request, &profile)?;
        let tree = resolve_recipe_tree(&tree_path, packages_subdir)?;
        Ok((tree, profile))
    }

    pub(crate) fn handle_host_scan_tree(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let (tree, profile) = self.resolve_host_tree(&request, None)?;
        let only = host_package_filter(&request);
        let names = discover_package_names(&tree, &only)?;
        let results = scan_tree_packages(self, &tree, &names)?;
        let ready = results
            .iter()
            .filter(|entry| entry.status == "ready")
            .count();
        let blocked = results.len() - ready;

        Ok(CommandReport {
            area: "host",
            status: if blocked == 0 { "ok" } else { "issues" },
            exit_status: if blocked == 0 {
                ExitStatus::Success
            } else {
                ExitStatus::OperatorFailure
            },
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "scanned {} package(s) under {} ({} ready, {} with issues).",
                results.len(),
                tree.packages_dir.display(),
                ready,
                blocked
            ),
            details: Some(json!({
                "profile": profile.name,
                "profile_path": profile.profile_path().display().to_string(),
                "scan": scan_tree_json(&tree.root, &results),
            })),
        })
    }

    pub(crate) fn handle_host_test_tree(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let install_smoke = request.operands.iter().any(|value| value == "--install");
        let (tree, profile) = self.resolve_host_tree(&request, None)?;
        let only = host_package_filter(&request);
        let names = discover_package_names(&tree, &only)?;
        let scan = scan_tree_packages(self, &tree, &names)?;
        let blockers: Vec<_> = scan
            .iter()
            .filter(|entry| entry.status != "ready")
            .map(|entry| entry.package.clone())
            .collect();
        if !blockers.is_empty() {
            return Ok(CommandReport {
                area: "host",
                status: "blocked",
                exit_status: ExitStatus::OperatorFailure,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "test-tree stopped: {} package(s) failed scan before planning.",
                    blockers.len()
                ),
                details: Some(json!({
                    "profile": profile.name,
                    "blocked_packages": blockers,
                    "scan": scan_tree_json(&tree.root, &scan),
                })),
            });
        }

        sync_tree_packages_to_recipes(&tree, &self.database.layout().recipes_dir, &names)?;
        let plan = publish_plan_for_targets(self, &names)?;
        let mut summary = format!(
            "test-tree planned {} package(s) across {} build layer(s) (dry-run checks).",
            plan.packages.len(),
            plan_max_layer(&plan.packages)
        );
        if install_smoke {
            summary.push_str(" Install smoke is not run in this slice; use `elda i` per package in a disposable root.");
        }

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary,
            details: Some(json!({
                "profile": profile.name,
                "install_smoke": install_smoke,
                "packages": plan.packages.iter().map(|pkg| json!({
                    "package": pkg.package_name,
                    "layer": pkg.layer,
                })).collect::<Vec<_>>(),
            })),
        })
    }

    pub(crate) fn handle_host_diff_tree(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let (tree, profile) = self.resolve_host_tree(&request, None)?;
        let since = parse_flag_value(&request, "--since").ok_or_else(|| {
            CoreError::Operator("`host diff-tree` requires `--since <git-ref>`".to_owned())
        })?;
        let changed = git_changed_packages_since(&tree, &since)?;

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "found {} changed package(s) since `{since}` under {}.",
                changed.len(),
                tree.packages_dir.display()
            ),
            details: Some(json!({
                "profile": profile.name,
                "since": since,
                "packages": changed,
            })),
        })
    }

    pub(crate) fn handle_host_client_bundle(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let profile_name = request
            .operands
            .first()
            .filter(|value| !value.starts_with("--"))
            .map(String::as_str)
            .map(String::from)
            .or(host_profile_name(&request));
        let profile = load_host_profile(&self.database.layout().root_dir, profile_name.as_deref())?;
        let channel = profile.default_channel();
        let base_url = profile
            .resolve_base_url()
            .unwrap_or_else(|| "https://example.invalid/elda".to_owned());
        let index_subpath = profile
            .channel_index_subpath(channel)
            .unwrap_or_else(|| channel.to_owned());
        let index_url = format!("{base_url}/{index_subpath}/index-v1.json.zst");
        let signature_url = format!("{index_url}.sig");
        let remote_name = profile.name.clone();

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("client bundle prepared for host profile `{remote_name}`."),
            details: Some(json!({
                "profile": profile.name,
                "remote": {
                    "name": remote_name,
                    "index_url": index_url,
                    "signature_url": signature_url,
                    "channel": channel,
                    "packages_url": profile.section.forge.remote.as_ref().and_then(|_| profile.tree_path()).map(|path| path.display().to_string()),
                },
                "commands": {
                    "rmt_add": format!("elda rmt add {remote_name}={index_url} --channel {channel}"),
                    "doctor": format!("elda doctor && elda host doctor --profile {}", profile.name),
                },
                "remotes_fragment": format!(
                    "# remotes.d/{remote_name}.toml\nname = \"{remote_name}\"\nindex_url = \"{index_url}\"\nsignature_url = \"{signature_url}\"\nchannel = \"{channel}\"\n"
                ),
            })),
        })
    }

    pub(crate) fn handle_host_status(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let profile = load_host_profile(
            &self.database.layout().root_dir,
            host_profile_name(&request).as_deref(),
        )?;
        let layout = self.database.layout();
        let mut channels: Vec<String> = profile.section.channels.keys().cloned().collect();
        let default_channel = profile.default_channel().to_owned();
        if !channels.iter().any(|name| name == &default_channel) {
            channels.push(default_channel.clone());
        }
        let mut channel_status = Vec::new();
        for channel in &channels {
            let workspace = CiWorkspacePaths::for_channel(layout, channel.as_str());
            let index_exists =
                workspace.index_path.is_file() || workspace.index_zst_path().is_file();
            channel_status.push(json!({
                "channel": channel,
                "published_dir": workspace.published_dir,
                "index_present": index_exists,
                "artifact_count": count_files(&workspace.artifacts_dir).unwrap_or(0),
            }));
        }

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("host status for profile `{}`.", profile.name),
            details: Some(json!({
                "profile": profile.name,
                "profile_path": profile.profile_path().display().to_string(),
                "tree": profile.tree_path(),
                "cache": {
                    "populate_after_publish": profile.cache_populate_after_publish(),
                    "upload_command_env": profile.cache_upload_command_env(),
                },
                "channels": channel_status,
            })),
        })
    }

    pub(crate) fn handle_host_doctor(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let profile = load_host_profile(
            &self.database.layout().root_dir,
            host_profile_name(&request).as_deref(),
        )?;
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        if profile.tree_path().is_none_or(|path| !path.is_dir()) {
            blockers.push("host.tree path is missing or not a directory".to_owned());
        }
        if profile
            .signing_key_path()
            .is_none_or(|path| !path.is_file())
        {
            warnings.push(
                "no host signing key configured; publish will use the CI workspace key".to_owned(),
            );
        }
        if profile.resolve_base_url().is_none() {
            warnings.push(
                "publish.base_url or publish.base_url_env is unset; finalize needs --base-url"
                    .to_owned(),
            );
        }

        Ok(CommandReport {
            area: "host",
            status: if blockers.is_empty() { "ok" } else { "issues" },
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
                "host doctor checked profile `{}` ({} blocker(s), {} warning(s)).",
                profile.name,
                blockers.len(),
                warnings.len()
            ),
            details: Some(json!({
                "profile": profile.name,
                "profile_path": profile.profile_path().display().to_string(),
                "blockers": blockers,
                "warnings": warnings,
            })),
        })
    }

    pub(crate) fn handle_host_init_ci(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let forge = parse_flag_value(&request, "--forge").unwrap_or_else(|| "github".to_owned());
        let workflow_path = PathBuf::from(".github/workflows/elda-publish.yml");
        if workflow_path.exists() && !request.operands.iter().any(|value| value == "--force") {
            return Err(CoreError::Operator(format!(
                "`{}` already exists; pass --force to overwrite",
                workflow_path.display()
            )));
        }
        if let Some(parent) = workflow_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&workflow_path, ci_workflow_template(&forge))?;

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "wrote CI workflow template to `{}`.",
                workflow_path.display()
            ),
            details: Some(json!({
                "workflow": workflow_path,
                "forge": forge,
            })),
        })
    }

    pub(crate) fn handle_host_link(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let tree_path = request
            .operands
            .first()
            .filter(|value| !value.starts_with("--"))
            .ok_or_else(|| {
                CoreError::Operator("`host link` requires a recipe tree path".to_owned())
            })?;
        let tree = resolve_recipe_tree(PathBuf::from(tree_path).as_path(), "packages")?;
        let workspace = CiWorkspacePaths::new(self.database.layout());
        workspace.ensure_exists()?;
        let names = discover_package_names(&tree, &[])?;
        sync_tree_packages_to_recipes(&tree, &workspace.packages_dir, &names)?;
        let commit = git_tree_commit(&tree.root)?;

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "linked {} package(s) from {} into the CI workspace.",
                names.len(),
                tree.root.display()
            ),
            details: Some(json!({
                "packages": names,
                "repo_commit": commit,
                "workspace": workspace.packages_dir,
            })),
        })
    }

    pub(crate) fn handle_host_push_recipes(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let profile = load_host_profile(
            &self.database.layout().root_dir,
            host_profile_name(&request).as_deref(),
        )?;
        let (tree, _) = self.resolve_host_tree(&request, Some(&profile))?;
        let remote = profile
            .section
            .forge
            .remote
            .clone()
            .unwrap_or_else(|| "origin".to_owned());
        let branch = profile.channel_branch(profile.default_channel());
        if request.dry_run {
            return Ok(CommandReport {
                area: "host",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: true,
                summary: format!(
                    "would push recipe tree `{}` to remote `{remote}` branch `{branch}`.",
                    tree.root.display()
                ),
                details: None,
            });
        }

        run_git_in(&tree.root, &["add", "."])?;
        let message = "elda host push-recipes".to_owned();
        let status = Command::new("git")
            .arg("-C")
            .arg(&tree.root)
            .args(["diff", "--cached", "--quiet"])
            .status()?;
        if !status.success() {
            run_git_in(&tree.root, &["commit", "-m", &message])?;
        }
        run_git_in(&tree.root, &["push", &remote, &branch])?;

        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("pushed recipe tree to `{remote}`/`{branch}`."),
            details: Some(json!({
                "remote": remote,
                "branch": branch,
                "tree": tree.root,
            })),
        })
    }

    pub(crate) fn handle_host_print_cache_config(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        let cache_name = request
            .operands
            .iter()
            .find(|value| !value.starts_with("--"))
            .cloned()
            .ok_or_else(|| {
                CoreError::Operator("`host print-cache-config` requires a cache name".to_owned())
            })?;
        let snippet = format!(
            r#"# Example static cache front for `{cache_name}`
location /elda-cache/ {{
    alias /var/cache/elda/{cache_name}/;
    autoindex off;
    add_header Cache-Control "public, max-age=31536000, immutable";
}}
"#
        );
        Ok(CommandReport {
            area: "host",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("printed example cache config for `{cache_name}`."),
            details: Some(json!({ "cache": cache_name, "snippet": snippet })),
        })
    }
}

fn host_profile_name(request: &CommandRequest) -> Option<String> {
    parse_flag_value(request, "--profile")
}

fn host_tree_path(
    request: &CommandRequest,
    profile: &ResolvedHostProfile,
) -> Result<PathBuf, CoreError> {
    if let Some(path) = request
        .operands
        .iter()
        .find(|value| !value.starts_with("--"))
        .map(PathBuf::from)
    {
        return Ok(path);
    }
    profile.tree_path().ok_or_else(|| {
        CoreError::Operator(
            "recipe tree path is required when host.tree is unset in the active profile".to_owned(),
        )
    })
}

fn host_package_filter(request: &CommandRequest) -> Vec<String> {
    let mut packages = Vec::new();
    let mut operands = request.operands.iter().peekable();
    while let Some(operand) = operands.next() {
        if operand == "--only" {
            while let Some(rest) = operands.peek() {
                if rest.starts_with("--") {
                    break;
                }
                packages.push(operands.next().expect("peeked").clone());
            }
        }
    }
    packages
}

fn parse_flag_value(request: &CommandRequest, flag: &str) -> Option<String> {
    let mut operands = request.operands.iter();
    while let Some(operand) = operands.next() {
        if operand == flag {
            return operands.next().cloned();
        }
    }
    None
}

fn plan_max_layer(packages: &[crate::app_ci::PlannedCiPackage]) -> u32 {
    packages.iter().map(|pkg| pkg.layer).max().unwrap_or(0)
}

fn count_files(directory: &std::path::Path) -> Result<usize, CoreError> {
    if !directory.is_dir() {
        return Ok(0);
    }
    Ok(fs::read_dir(directory)?.count())
}

fn run_git_in(repo: &std::path::Path, args: &[&str]) -> Result<(), CoreError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(CoreError::Operator(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )))
    }
}

fn ci_workflow_template(forge: &str) -> String {
    format!(
        r#"name: Elda publish

on:
  push:
    branches: [main, staging, unstable]
    tags: ['index-*']

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Elda
        run: echo "install elda from your release channel"
      - name: Scan recipe tree
        run: elda host scan-tree .
      - name: Test recipe tree (dry-run)
        run: elda host test-tree .
      - name: Publish channel
        env:
          ELDA_PUBLISH_BASE_URL: ${{{{ secrets.ELDA_PUBLISH_BASE_URL }}}}
          ELDA_SIGNING_KEY: ${{{{ secrets.ELDA_SIGNING_KEY }}}}
        run: |
          elda publish run --tree . --channel stable
          elda publish finalize --channel stable --base-url "$ELDA_PUBLISH_BASE_URL"
      - name: Upload artifacts
        run: echo "upload data/ci/published-stable to your static host ({forge})"
"#
    )
}
