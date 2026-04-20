use super::*;

impl AppContext {
    pub(crate) fn handle_remove(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = self.parse_remove_request(&request)?;
        let packages = self.removal_order(&parsed.packages, parsed.cascade)?;
        let mutation_policy = self.mutation_policy();

        if request.dry_run {
            let actions = packages
                .iter()
                .map(|package| json!({ "target": package, "action": "remove" }))
                .collect::<Vec<_>>();
            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!("planned removal of {} package(s).", packages.len()),
                details: Some(json!({ "plan": { "kind": "remove", "actions": actions } })),
            });
        }

        let mut removals = Vec::new();
        for package in packages {
            let report = if parsed.purge_conffiles {
                remove_package_purge_conffiles(&self.database, &package, &mutation_policy)?
            } else {
                remove_package(&self.database, &package, &mutation_policy)?
            };
            removals.push(report);
        }
        let _ = self.reconcile_cache_policy()?;

        Ok(CommandReport {
            area: "remove",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("removed {} package(s).", removals.len()),
            details: Some(json!({ "removals": removals })),
        })
    }

    pub(crate) fn handle_autoremove_plan(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let packages = self.orphan_candidates()?;
        let mutation_policy = self.mutation_policy();
        let actions = packages
            .iter()
            .map(|package| json!({ "target": package, "action": "autoremove" }))
            .collect::<Vec<_>>();

        if request.dry_run {
            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!("planned autoremove of {} package(s).", packages.len()),
                details: Some(json!({ "plan": { "kind": "autoremove", "actions": actions } })),
            });
        }

        let mut removals = Vec::new();
        for package in packages {
            removals.push(remove_package(&self.database, &package, &mutation_policy)?);
        }
        let _ = self.reconcile_cache_policy()?;

        Ok(CommandReport {
            area: "remove",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("autoremoved {} package(s).", removals.len()),
            details: Some(json!({ "removals": removals })),
        })
    }

    fn removal_order(&self, requested: &[String], cascade: bool) -> Result<Vec<String>, CoreError> {
        let mut order = Vec::new();
        let mut seen = BTreeSet::new();

        for package in requested {
            self.ensure_installed(package)?;
            let reverse_dependencies = self.database.reverse_dependencies(package, false)?;
            if !cascade && !reverse_dependencies.is_empty() {
                return Err(CoreError::Operator(format!(
                    "package `{package}` is still required; use `--cascade` to remove reverse dependencies"
                )));
            }
            self.collect_removal_order(package, cascade, &mut seen, &mut order)?;
        }

        Ok(order)
    }

    fn collect_removal_order(
        &self,
        package: &str,
        cascade: bool,
        seen: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if !seen.insert(package.to_owned()) {
            return Ok(());
        }
        if cascade {
            for reverse in self.database.reverse_dependencies(package, false)? {
                self.collect_removal_order(&reverse.pkgname, true, seen, order)?;
            }
        }
        order.push(package.to_owned());

        Ok(())
    }
}
