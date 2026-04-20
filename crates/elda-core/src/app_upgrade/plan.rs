use super::*;

impl AppContext {
    pub(crate) fn handle_upgrade(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = self.parse_upgrade_request(&request)?;
        let targets = if parsed.targets.is_empty() {
            self.database.state_snapshot()?.world
        } else {
            parsed.targets.clone()
        };
        let install_request = ParsedInstallRequest {
            targets: targets.clone(),
            hard_lane: None,
            preferred_lane: None,
            cli_flag_overrides: Default::default(),
        };
        let plan =
            self.plan_upgrade_targets(&targets, &install_request, parsed.refresh_weak_deps)?;
        self.validate_upgrade_conflicts(&plan)?;
        self.validate_upgrade_coherence(&plan)?;
        let actions = plan
            .iter()
            .map(Self::upgrade_action_json)
            .collect::<Result<Vec<_>, _>>()?;

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

        let upgrades = self.apply_upgrade_plan(&plan, request.offline)?;

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

    pub(crate) fn plan_upgrade_targets(
        &self,
        targets: &[String],
        request: &ParsedInstallRequest,
        refresh_weak_deps: bool,
    ) -> Result<Vec<PlannedUpgradeAction>, CoreError> {
        let solved = self.solve_upgrade_request(request, targets, refresh_weak_deps)?;
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
            });
        }

        Ok(actions)
    }

    fn validate_upgrade_conflicts(
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

    fn upgrade_action_json(action: &PlannedUpgradeAction) -> Result<serde_json::Value, CoreError> {
        let decision = Self::decision_for_action(action)?;

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
            "variant_id": action.resolved.flag_state.variant_id,
            "needs_change": decision.needs_change,
            "replaced_packages": action.replaced_packages,
            "blocked_reason": decision.blocked_reason,
            "pinned_version": decision.pinned_version,
            "hold_source": decision.hold_source,
        }))
    }
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
