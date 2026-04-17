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
        let mut actions = Vec::new();
        let mut planned_by_package = BTreeMap::new();
        let mut visiting = BTreeSet::new();

        for target in targets {
            self.plan_upgrade_target_closure(
                target,
                request,
                refresh_weak_deps,
                true,
                None,
                None,
                &mut visiting,
                &mut planned_by_package,
                &mut actions,
            )?;
        }

        Ok(actions)
    }

    fn plan_upgrade_target_closure(
        &self,
        target: &str,
        request: &ParsedInstallRequest,
        refresh_weak_deps: bool,
        explicit_target: bool,
        requested_by: Option<&str>,
        resolved_by: Option<&ResolvedDependencyPlan>,
        visiting: &mut BTreeSet<String>,
        planned_by_package: &mut BTreeMap<String, usize>,
        actions: &mut Vec<PlannedUpgradeAction>,
    ) -> Result<(), CoreError> {
        let installed = self.ensure_installed(target).ok();
        let resolved = self.resolve_install_target(target, request)?;
        let package_name = resolved.recipe.package.name.clone();

        if let Some(index) = planned_by_package.get(&package_name).copied() {
            if explicit_target {
                actions[index].explicit_target = true;
            }
            return Ok(());
        }

        if !visiting.insert(package_name.clone()) {
            return Err(CoreError::Operator(format!(
                "dependency cycle detected while planning upgrade for `{package_name}`"
            )));
        }

        let dependencies = if explicit_target && refresh_weak_deps {
            self.collect_install_dependencies(&resolved.recipe.package, "explicit", request)?
        } else {
            self.collect_required_dependencies(&resolved.recipe.package, request)?
        };
        let replaced_packages = self.planned_replacements(&resolved)?;
        for dependency in &dependencies {
            self.plan_upgrade_target_closure(
                &dependency.target,
                &self.dependency_install_request(request),
                false,
                false,
                Some(&package_name),
                Some(dependency),
                visiting,
                planned_by_package,
                actions,
            )?;
        }
        visiting.remove(&package_name);

        let install_reason = installed
            .as_ref()
            .map(|package| package.install_reason.clone())
            .unwrap_or_else(|| "dep".to_owned());
        planned_by_package.insert(package_name.clone(), actions.len());
        actions.push(PlannedUpgradeAction {
            package_name,
            resolved,
            replaced_packages,
            install_reason,
            requested_by: requested_by.map(ToOwned::to_owned),
            dependency_kind: resolved_by.map(|dependency| dependency.dependency_kind.clone()),
            raw_expr: resolved_by.map(|dependency| dependency.raw_expr.clone()),
            dependencies,
            installed,
            explicit_target,
        });

        Ok(())
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
