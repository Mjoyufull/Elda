use super::*;

use crate::app_confirm::confirm_mutation;
use crate::app_install::parse_ad_hoc_git_source_ref;

impl AppContext {
    pub(crate) fn handle_upgrade(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let mut parsed = self.parse_upgrade_request(&request)?;
        crate::app_install::git_picker::apply_upgrade_pick_tag_selection(
            self,
            &request,
            &mut parsed,
        )?;
        let targets = if parsed.rebuild_variant_drift {
            self.collect_variant_drift_targets(&parsed.targets)?
        } else if parsed.targets.is_empty() {
            self.database.state_snapshot()?.world
        } else {
            parsed.targets.clone()
        };
        let selection = self.upgrade_install_request(&targets, &parsed)?;
        let plan_targets = selection.request.targets.clone();
        let mut plan = self.plan_upgrade_targets(
            &plan_targets,
            &selection.request,
            parsed.refresh_weak_deps,
            parsed.rebuild_variant_drift,
            Some(&request),
        )?;
        self.populate_ad_hoc_candidate_commits(&mut plan, request.offline)?;
        self.validate_upgrade_conflicts(&plan)?;
        self.validate_upgrade_coherence(&plan)?;
        let mut actions = selection
            .skipped_actions
            .iter()
            .map(skipped_git_action_json)
            .collect::<Vec<_>>();
        actions.extend(
            plan.iter()
                .map(Self::upgrade_action_json)
                .collect::<Result<Vec<_>, _>>()?,
        );

        if request.dry_run {
            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "planned upgrade evaluation for {} package(s).",
                    targets.len()
                ),
                details: Some(json!({ "plan": { "kind": "upgrade", "actions": actions } })),
            });
        }

        confirm_mutation(
            &request,
            &format!("Apply upgrade plan for {} package action(s)?", plan.len()),
        )?;
        let upgrades = self.apply_upgrade_plan(&plan, &request)?;

        Ok(CommandReport {
            area: "upgrade",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("evaluated upgrade state for {} package(s).", plan.len()),
            details: Some(json!({
                "actions": actions,
                "upgrades": upgrades,
            })),
        })
    }

    pub(super) fn collect_variant_drift_targets(
        &self,
        explicit: &[String],
    ) -> Result<Vec<String>, CoreError> {
        let allowlist: Option<BTreeSet<String>> = if explicit.is_empty() {
            None
        } else {
            Some(explicit.iter().cloned().collect())
        };

        let installed = self.database.list_installed_packages()?;
        let mut targets = Vec::new();
        let mut seen = BTreeSet::new();

        for package in installed {
            if let Some(filter) = allowlist.as_ref()
                && !filter.contains(&package.pkgname)
            {
                continue;
            }
            let resolved = match self
                .resolve_install_target(&package.pkgname, &ParsedInstallRequest::default())
            {
                Ok(resolved) => resolved,
                Err(_) => continue,
            };
            let installed_variant = package
                .variant_id
                .clone()
                .unwrap_or_else(|| "default".to_owned());
            if installed_variant != resolved.flag_state.variant_id
                && seen.insert(package.pkgname.clone())
            {
                targets.push(package.pkgname);
            }
        }

        Ok(targets)
    }

    fn upgrade_install_request(
        &self,
        targets: &[String],
        parsed: &crate::app::ParsedUpgradeRequest,
    ) -> Result<UpgradeInstallSelection, CoreError> {
        let mut request = ParsedInstallRequest {
            targets: targets.to_vec(),
            git_ref: parsed.git_ref.clone(),
            ..Default::default()
        };
        let mut skipped_actions = Vec::new();

        for target in targets {
            let Ok(installed) = self.ensure_installed(target) else {
                continue;
            };
            if installed.source_kind != "git" {
                continue;
            }

            let Some(source_ref) = installed.source_ref.clone() else {
                continue;
            };
            let parsed_ref = parse_ad_hoc_git_source_ref(&source_ref);
            let git_ref = parsed.git_ref.clone().or(parsed_ref.git_ref.clone());
            if parsed.git_ref.is_none() && !parsed_ref.moving {
                skipped_actions.push(SkippedGitUpgradeAction {
                    package: installed,
                    source_ref: source_ref.clone(),
                });
                request
                    .git_source_refs
                    .insert(target.clone(), source_ref.to_owned());
                request.targets.retain(|value| value != target);
                continue;
            }
            request
                .git_source_refs
                .insert(target.clone(), parsed_ref.target.clone());
            if let Some(git_ref) = git_ref {
                request.git_ref_overrides.insert(target.clone(), git_ref);
            }
        }

        if request.git_ref.is_some() && request.git_source_refs.is_empty() {
            return Err(CoreError::Operator(
                "git ref switching requires installed ad hoc git target(s)".to_owned(),
            ));
        }

        Ok(UpgradeInstallSelection {
            request,
            skipped_actions,
        })
    }

    pub(super) fn populate_ad_hoc_candidate_commits(
        &self,
        actions: &mut [PlannedUpgradeAction],
        offline: bool,
    ) -> Result<(), CoreError> {
        for action in actions {
            if !action.resolved.ad_hoc_git {
                continue;
            }
            let Some(installed) = &action.installed else {
                continue;
            };
            if installed.held || installed.pinned_version.is_some() {
                continue;
            }
            let built = self.build_resolved_target(&action.resolved, offline, false, None)?;
            action.candidate_repo_commit = built.package.repo_commit;
        }

        Ok(())
    }

    pub(crate) fn plan_upgrade_targets(
        &self,
        targets: &[String],
        request: &ParsedInstallRequest,
        refresh_weak_deps: bool,
        rebuild_variant_drift: bool,
        command: Option<&crate::CommandRequest>,
    ) -> Result<Vec<PlannedUpgradeAction>, CoreError> {
        let solved = self.solve_upgrade_request(request, targets, refresh_weak_deps, command)?;
        let explicit_targets = solved
            .explicit_targets
            .values()
            .cloned()
            .collect::<BTreeSet<_>>();
        let dependency_origins = dependency_origins(&solved.order, &solved.packages);
        let mut actions = Vec::with_capacity(solved.order.len());

        for package_name in &solved.order {
            let package = solved
                .packages
                .get(package_name)
                .expect("solved package should exist in the final order");
            let installed = self.ensure_installed(package_name).ok();
            let replaced_packages = self.planned_replacements(&package.resolved)?;
            let origin = dependency_origins.get(package_name);

            actions.push(PlannedUpgradeAction {
                package_name: package_name.clone(),
                resolved: package.resolved.clone(),
                replaced_packages,
                install_reason: installed
                    .as_ref()
                    .map(|package| package.install_reason.clone())
                    .unwrap_or_else(|| "dep".to_owned()),
                requested_by: origin.map(|origin| origin.requested_by.clone()),
                dependency_kind: origin.map(|origin| origin.dependency_kind.clone()),
                raw_expr: origin.map(|origin| origin.raw_expr.clone()),
                dependencies: package.dependencies.clone(),
                installed,
                explicit_target: explicit_targets.contains(package_name),
                candidate_repo_commit: None,
                rebuild_variant_drift,
            });
        }

        Ok(actions)
    }

    pub(super) fn validate_upgrade_conflicts(
        &self,
        actions: &[PlannedUpgradeAction],
    ) -> Result<(), CoreError> {
        let planned_packages = actions
            .iter()
            .filter_map(|action| match self.upgrade_decision(action) {
                Ok(decision) if decision.needs_change => Some(Ok(action.package_name.clone())),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<BTreeSet<_>, _>>()?;
        let removed_packages = actions
            .iter()
            .flat_map(|action| action.replaced_packages.iter().cloned())
            .collect::<BTreeSet<_>>();
        let installed_packages = self
            .database
            .list_installed_packages()?
            .into_iter()
            .map(|package| package.pkgname)
            .collect::<BTreeSet<_>>();

        for action in actions {
            let decision = self.upgrade_decision(action)?;
            if !decision.needs_change {
                continue;
            }
            self.validate_conflict_set(
                &action.package_name,
                &action.resolved.recipe.package.conflicts,
                &planned_packages,
                &removed_packages,
                &installed_packages,
            )?;
        }

        Ok(())
    }

    pub(super) fn upgrade_action_json(
        action: &PlannedUpgradeAction,
    ) -> Result<serde_json::Value, CoreError> {
        let decision = Self::decision_for_action(action)?;
        let installed_variant = action
            .installed
            .as_ref()
            .and_then(|installed| installed.variant_id.clone());
        let resolved_variant = action.resolved.flag_state.variant_id.clone();
        let variant_changed = installed_variant
            .as_deref()
            .is_some_and(|installed| installed != resolved_variant);

        Ok(json!({
            "target": action.package_name,
            "requested_by": action.requested_by,
            "dependency_kind": action.dependency_kind,
            "raw_expr": action.raw_expr,
            "explicit_target": action.explicit_target,
            "action": decision.change_kind,
            "installed_version": decision.installed_version,
            "candidate_version": decision.candidate_version,
            "selected_lane": decision.selected_lane,
            "selected_source_kind": action.resolved.selected_source_kind,
            "persisted_source_kind": action.resolved.persisted_source_kind,
            "source_ref": action.resolved.source_ref,
            "ad_hoc_git_moving": action.resolved.ad_hoc_git_moving,
            "installed_repo_commit": action.installed.as_ref().and_then(|package| package.repo_commit.clone()),
            "candidate_repo_commit": action.candidate_repo_commit,
            "variant_id": resolved_variant,
            "installed_variant_id": installed_variant,
            "variant_changed": variant_changed,
            "rebuild_variant_drift": action.rebuild_variant_drift,
            "needs_change": decision.needs_change,
            "replaced_packages": action.replaced_packages,
            "blocked_reason": decision.blocked_reason,
            "pinned_version": decision.pinned_version,
            "hold_source": decision.hold_source,
        }))
    }
}

#[derive(Debug, Clone)]
struct UpgradeInstallSelection {
    request: ParsedInstallRequest,
    skipped_actions: Vec<SkippedGitUpgradeAction>,
}

#[derive(Debug, Clone)]
struct SkippedGitUpgradeAction {
    package: elda_db::InstalledPackageDetails,
    source_ref: String,
}

fn skipped_git_action_json(action: &SkippedGitUpgradeAction) -> serde_json::Value {
    let version = installed_version(&action.package);

    json!({
        "target": action.package.pkgname,
        "package": action.package.pkgname,
        "explicit_target": true,
        "action": "keep-installed",
        "installed_version": version,
        "candidate_version": version,
        "version": version,
        "selected_lane": "source",
        "selected_source_kind": "git",
        "persisted_source_kind": "git",
        "source_ref": action.source_ref,
        "ad_hoc_git_moving": false,
        "installed_repo_commit": action.package.repo_commit,
        "candidate_repo_commit": null,
        "variant_id": action.package.variant_id,
        "needs_change": false,
        "replaced_packages": [],
        "blocked_reason": "git-ref-pinned",
        "pinned_version": action.package.pinned_version,
        "hold_source": action.package.hold_source,
    })
}

#[derive(Debug, Clone)]
struct DependencyOrigin {
    requested_by: String,
    dependency_kind: String,
    raw_expr: String,
}

fn dependency_origins(
    order: &[String],
    packages: &BTreeMap<String, crate::app_install::solver::SolvedPackage>,
) -> BTreeMap<String, DependencyOrigin> {
    let mut origins = BTreeMap::new();

    for package_name in order.iter().rev() {
        let Some(package) = packages.get(package_name) else {
            continue;
        };
        for dependency in &package.dependencies {
            origins
                .entry(dependency.target.clone())
                .or_insert_with(|| DependencyOrigin {
                    requested_by: package.package_name.clone(),
                    dependency_kind: dependency.dependency_kind.clone(),
                    raw_expr: dependency.raw_expr.clone(),
                });
        }
    }

    origins
}
