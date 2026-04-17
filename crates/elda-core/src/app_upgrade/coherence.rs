use super::*;

use crate::app_install::{
    package_satisfies_constraint, parse_dependency_constraint, provides_satisfy_constraint,
};

impl AppContext {
    pub(super) fn validate_upgrade_coherence(
        &self,
        actions: &[PlannedUpgradeAction],
    ) -> Result<(), CoreError> {
        let planned = actions
            .iter()
            .enumerate()
            .map(|(index, action)| (action.package_name.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let removed = actions
            .iter()
            .flat_map(|action| action.replaced_packages.iter().cloned())
            .collect::<BTreeSet<_>>();

        self.validate_planned_dependency_coherence(actions, &planned)?;
        self.validate_installed_reverse_dependency_coherence(actions, &planned, &removed)?;

        Ok(())
    }

    fn validate_planned_dependency_coherence(
        &self,
        actions: &[PlannedUpgradeAction],
        planned: &BTreeMap<String, usize>,
    ) -> Result<(), CoreError> {
        for action in actions {
            for dependency in &action.dependencies {
                let Some(index) = planned.get(&dependency.target).copied() else {
                    continue;
                };
                let dependency_action = &actions[index];
                let dependency_decision = self.upgrade_decision(dependency_action)?;
                if !dependency_decision.needs_change {
                    continue;
                }
                if candidate_satisfies_dependency(
                    &dependency_action.resolved.recipe.package,
                    &dependency.raw_expr,
                    dependency.provider_group.as_deref(),
                )? {
                    continue;
                }

                return Err(CoreError::Operator(format!(
                    "planned package `{}` requires `{}`, but `{}` is planned as {}",
                    action.package_name,
                    dependency.raw_expr,
                    dependency_action.package_name,
                    dependency_decision.candidate_version,
                )));
            }
        }

        Ok(())
    }

    fn validate_installed_reverse_dependency_coherence(
        &self,
        actions: &[PlannedUpgradeAction],
        planned: &BTreeMap<String, usize>,
        removed: &BTreeSet<String>,
    ) -> Result<(), CoreError> {
        for action in actions {
            let decision = self.upgrade_decision(action)?;
            if !decision.needs_change {
                continue;
            }

            for reverse in self
                .database
                .reverse_dependencies(&action.package_name, false)?
            {
                if planned.contains_key(&reverse.pkgname) || removed.contains(&reverse.pkgname) {
                    continue;
                }
                if candidate_satisfies_dependency(
                    &action.resolved.recipe.package,
                    &reverse.raw_expr,
                    reverse.provider_group.as_deref(),
                )? {
                    continue;
                }

                return Err(CoreError::Operator(format!(
                    "cannot upgrade `{}` to {} because installed package `{}` requires `{}`",
                    action.package_name,
                    decision.candidate_version,
                    reverse.pkgname,
                    reverse.raw_expr,
                )));
            }
        }

        Ok(())
    }
}

fn candidate_satisfies_dependency(
    package: &elda_recipe::PackageDefinition,
    raw_expr: &str,
    provider_group: Option<&str>,
) -> Result<bool, CoreError> {
    if let Some(provider_group) = provider_group {
        for provider in provider_group.split(" | ") {
            if candidate_matches_constraint(package, provider)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }

    candidate_matches_constraint(package, raw_expr)
}

fn candidate_matches_constraint(
    package: &elda_recipe::PackageDefinition,
    constraint_expr: &str,
) -> Result<bool, CoreError> {
    let constraint = parse_dependency_constraint(constraint_expr)?;

    Ok(package_satisfies_constraint(package, &constraint)
        || provides_satisfy_constraint(&package.provides, &constraint)?)
}
