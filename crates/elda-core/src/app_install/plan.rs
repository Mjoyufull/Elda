use std::collections::{BTreeMap, BTreeSet};

use crate::app::{AppContext, ParsedInstallRequest, PlannedInstallAction, ResolvedDependencyPlan};
use crate::app_parse::dependency_name_from_constraint;
use crate::error::CoreError;

impl AppContext {
    pub(crate) fn plan_install_targets(
        &self,
        request: &ParsedInstallRequest,
    ) -> Result<Vec<PlannedInstallAction>, CoreError> {
        let mut actions = Vec::new();
        let mut planned_by_package = BTreeMap::new();
        let mut visiting = BTreeSet::new();

        for target in &request.targets {
            self.plan_install_target_closure(
                target,
                request,
                "explicit",
                None,
                None,
                &mut visiting,
                &mut planned_by_package,
                &mut actions,
            )?;
        }

        Ok(actions)
    }

    pub(crate) fn plan_install_target_closure(
        &self,
        target: &str,
        request: &ParsedInstallRequest,
        install_reason: &str,
        requested_by: Option<&str>,
        resolved_by: Option<&ResolvedDependencyPlan>,
        visiting: &mut BTreeSet<String>,
        planned_by_package: &mut BTreeMap<String, usize>,
        actions: &mut Vec<PlannedInstallAction>,
    ) -> Result<(), CoreError> {
        let dependency_request = self.dependency_install_request(request);
        let selected_request = if install_reason == "explicit" {
            request
        } else {
            &dependency_request
        };
        let resolved = self.resolve_install_target(target, selected_request)?;
        let package_name = resolved.recipe.package.name.clone();

        if let Some(index) = planned_by_package.get(&package_name).copied() {
            if install_reason == "explicit" && actions[index].install_reason != "explicit" {
                actions[index].install_reason = "explicit".to_owned();
                actions[index].requested_by = None;
                actions[index].dependency_kind = None;
                actions[index].raw_expr = None;
                actions[index].is_weak = false;
                actions[index].provider_group = None;
            }
            return Ok(());
        }

        if !visiting.insert(package_name.clone()) {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("dependency cycle detected while planning `{package_name}`"),
            )));
        }

        let dependencies = self.collect_install_dependencies(
            &resolved.recipe.package,
            install_reason,
            &dependency_request,
        )?;
        let replaced_packages = self.planned_replacements(&resolved)?;
        for dependency in &dependencies {
            self.plan_install_target_closure(
                &dependency.target,
                &dependency_request,
                "dep",
                Some(&package_name),
                Some(dependency),
                visiting,
                planned_by_package,
                actions,
            )?;
        }
        visiting.remove(&package_name);

        let already_installed = self.database.installed_package(&package_name)?;
        planned_by_package.insert(package_name.clone(), actions.len());
        actions.push(PlannedInstallAction {
            target: target.to_owned(),
            package_name,
            resolved,
            replaced_packages,
            install_reason: install_reason.to_owned(),
            requested_by: requested_by.map(ToOwned::to_owned),
            dependency_kind: resolved_by.map(|dependency| dependency.dependency_kind.clone()),
            raw_expr: resolved_by.map(|dependency| dependency.raw_expr.clone()),
            is_weak: resolved_by.is_some_and(|dependency| dependency.is_weak),
            provider_group: resolved_by.and_then(|dependency| dependency.provider_group.clone()),
            dependencies,
            already_installed,
        });

        Ok(())
    }

    pub(crate) fn validate_install_conflicts(
        &self,
        actions: &[PlannedInstallAction],
    ) -> Result<(), CoreError> {
        let planned_packages = actions
            .iter()
            .map(|action| action.package_name.clone())
            .collect::<BTreeSet<_>>();
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

    pub(crate) fn validate_conflict_set(
        &self,
        package_name: &str,
        conflicts: &[String],
        planned_packages: &BTreeSet<String>,
        removed_packages: &BTreeSet<String>,
        installed_packages: &BTreeSet<String>,
    ) -> Result<(), CoreError> {
        for conflict in conflicts {
            let conflict_name = dependency_name_from_constraint(conflict);
            if conflict_name == package_name {
                continue;
            }
            if planned_packages.contains(&conflict_name) {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!(
                        "package `{package_name}` conflicts with `{conflict_name}` in the same transaction plan"
                    ),
                )));
            }
            if removed_packages.contains(&conflict_name) {
                continue;
            }
            if installed_packages.contains(&conflict_name) {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!(
                        "package `{package_name}` conflicts with installed package `{conflict_name}`"
                    ),
                )));
            }
        }

        Ok(())
    }
}
