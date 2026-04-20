use std::collections::{BTreeMap, BTreeSet};

use crate::app::{AppContext, ParsedInstallRequest, PlannedInstallAction};
use crate::app_parse::dependency_name_from_constraint;
use crate::error::CoreError;

impl AppContext {
    pub(crate) fn plan_install_targets(
        &self,
        request: &ParsedInstallRequest,
    ) -> Result<Vec<PlannedInstallAction>, CoreError> {
        let solved = self.solve_install_request(request)?;
        let explicit_targets = solved.explicit_targets.iter().fold(
            BTreeMap::<String, String>::new(),
            |mut targets, (target, package_name)| {
                targets
                    .entry(package_name.clone())
                    .or_insert_with(|| target.clone());
                targets
            },
        );
        let dependency_origins = dependency_origins(&solved.order, &solved.packages);
        let mut actions = Vec::with_capacity(solved.order.len());

        for package_name in &solved.order {
            let package = solved
                .packages
                .get(package_name)
                .expect("solved package should exist in the final order");
            let replaced_packages = self.planned_replacements(&package.resolved)?;
            let already_installed = self.database.installed_package(package_name)?;
            let explicit_target = explicit_targets.get(package_name);
            let origin = dependency_origins.get(package_name);

            actions.push(PlannedInstallAction {
                target: explicit_target
                    .cloned()
                    .unwrap_or_else(|| package_name.clone()),
                package_name: package_name.clone(),
                resolved: package.resolved.clone(),
                replaced_packages,
                install_reason: if explicit_target.is_some() {
                    "explicit".to_owned()
                } else {
                    "dep".to_owned()
                },
                requested_by: origin.map(|origin| origin.requested_by.clone()),
                dependency_kind: origin.map(|origin| origin.dependency_kind.clone()),
                raw_expr: origin.map(|origin| origin.raw_expr.clone()),
                is_weak: origin.is_some_and(|origin| origin.is_weak),
                provider_group: origin.and_then(|origin| origin.provider_group.clone()),
                dependencies: package.dependencies.clone(),
                already_installed,
            });
        }

        Ok(actions)
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

#[derive(Debug, Clone)]
struct DependencyOrigin {
    requested_by: String,
    dependency_kind: String,
    raw_expr: String,
    is_weak: bool,
    provider_group: Option<String>,
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
                    is_weak: dependency.is_weak,
                    provider_group: dependency.provider_group.clone(),
                });
        }
    }

    origins
}
