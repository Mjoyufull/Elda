use serde::Serialize;

use super::*;

impl AppContext {
    pub(crate) fn apply_upgrade_plan(
        &self,
        actions: &[PlannedUpgradeAction],
        offline: bool,
    ) -> Result<Vec<serde_json::Value>, CoreError> {
        let mut upgrades = Vec::new();
        let mutation_policy = self.mutation_policy();

        for action in actions {
            let decision = self.upgrade_decision(action)?;
            if !decision.needs_change {
                continue;
            }

            let mut built = self.build_resolved_target(&action.resolved, offline)?;
            built.package.dependencies = Self::planned_dependency_records(&action.dependencies);
            let mut replaced = Vec::new();
            for package_name in &action.replaced_packages {
                replaced.push(serde_json::to_value(remove_package_for_upgrade(
                    &self.database,
                    package_name,
                    &mutation_policy,
                )?)?);
            }

            if let Some(installed) = &action.installed {
                remove_package_for_upgrade(&self.database, &action.package_name, &mutation_policy)?;
                let report = install_upgraded_package(
                    &self.database,
                    &built.package,
                    &action.install_reason,
                    installed.pinned_version.clone(),
                    installed.held,
                    installed.hold_source.clone(),
                    &mutation_policy,
                )?;
                upgrades.push(upgrade_json(action, &decision, report, replaced));
            } else {
                let report = install_built_package(
                    &self.database,
                    &built.package,
                    &action.install_reason,
                    None,
                    false,
                    None,
                    &mutation_policy,
                )?;
                upgrades.push(upgrade_json(action, &decision, report, replaced));
            }
        }
        let _ = self.reconcile_cache_policy()?;

        Ok(upgrades)
    }
}

fn upgrade_json(
    action: &PlannedUpgradeAction,
    decision: &UpgradeDecision,
    report: impl Serialize,
    replaced: Vec<serde_json::Value>,
) -> serde_json::Value {
    json!({
        "target": action.package_name,
        "action": decision.change_kind,
        "install": report,
        "replacements": replaced,
        "selected_lane": action.resolved.selected_lane,
        "selected_source_kind": action.resolved.selected_source_kind,
        "persisted_source_kind": action.resolved.persisted_source_kind,
        "candidate_version": decision.candidate_version,
        "variant_id": action.resolved.flag_state.variant_id,
        "effective_flags": action.resolved.flag_state.effective_flags,
    })
}
