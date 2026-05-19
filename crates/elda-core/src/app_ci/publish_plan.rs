use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::app::AppContext;
use crate::error::CoreError;
use elda_recipe::{
    DependencyBody, DependencyEntry, add_recipe_with_priority, is_git_like_target, load_recipe,
};
use elda_types::NamedConstraint;

#[derive(Debug, Clone)]
pub(crate) struct PlannedCiPackage {
    pub(crate) package_name: String,
    pub(crate) recipe_path: PathBuf,
    pub(crate) runtime_depends: Vec<String>,
    pub(crate) makedepends: Vec<String>,
    pub(crate) checkdepends: Vec<String>,
    pub(crate) layer: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct PlannedCiWork {
    pub(crate) requested_targets: Vec<String>,
    pub(crate) packages: Vec<PlannedCiPackage>,
}

pub(crate) fn resolve_ci_targets(
    app: &AppContext,
    targets: &[String],
) -> Result<Vec<String>, CoreError> {
    if targets.is_empty() {
        return Err(CoreError::Operator(
            "ci requires at least one package, batch, path, or git target".to_owned(),
        ));
    }

    let mut resolved = Vec::new();
    for target in targets {
        let target_path = PathBuf::from(target);
        if target_path.exists() || is_git_like_target(target) {
            let report = match add_recipe_with_priority(
                &app.database.layout().recipes_dir,
                target,
                None,
                &app.config.metadata.link_strategy_priority,
                &app.config.metadata.release_binary_format_priority,
            )? {
                elda_recipe::ImportResult::Single(r) => r,
                elda_recipe::ImportResult::Bulk(_) => {
                    return Err(CoreError::Operator("Bulk imports are not supported in CI planning. Use `elda add <url>` first.".to_owned()));
                }
            };
            resolved.push(report.recipe_name);
            continue;
        }

        let recipe_dir = app.database.layout().recipes_dir.join(target);
        if !recipe_dir.join("pkg.lua").is_file() {
            return Err(CoreError::Operator(format!(
                "local recipe `{target}` does not exist under {}",
                app.database.layout().recipes_dir.display()
            )));
        }
        resolved.push(target.clone());
    }

    resolved.sort();
    resolved.dedup();
    Ok(resolved)
}

pub(crate) fn plan_ci_work(
    app: &AppContext,
    requested_targets: &[String],
) -> Result<PlannedCiWork, CoreError> {
    let mut queue = VecDeque::from(requested_targets.to_vec());
    let mut seen = BTreeSet::new();
    let mut packages = BTreeMap::new();

    while let Some(package_name) = queue.pop_front() {
        if !seen.insert(package_name.clone()) {
            continue;
        }

        let recipe = load_recipe(&app.database.layout().recipes_dir, &package_name)?;
        let runtime_local =
            local_dependency_targets(&app.database.layout().recipes_dir, &recipe.package.depends)?;
        let make_local = local_dependency_targets(
            &app.database.layout().recipes_dir,
            &recipe.package.makedepends,
        )?;
        let check_local = local_dependency_targets(
            &app.database.layout().recipes_dir,
            &recipe.package.checkdepends,
        )?;

        for dependency in runtime_local
            .iter()
            .chain(make_local.iter())
            .chain(check_local.iter())
        {
            queue.push_back(dependency.clone());
        }

        packages.insert(
            package_name.clone(),
            PlannedCiPackage {
                package_name,
                recipe_path: recipe.path.clone(),
                runtime_depends: dependency_strings(&recipe.package.depends),
                makedepends: dependency_strings(&recipe.package.makedepends),
                checkdepends: dependency_strings(&recipe.package.checkdepends),
                layer: 0,
            },
        );
    }

    assign_layers(app, &mut packages)?;

    let mut planned = packages.into_values().collect::<Vec<_>>();
    planned.sort_by(|left, right| {
        left.layer
            .cmp(&right.layer)
            .then_with(|| left.package_name.cmp(&right.package_name))
    });

    Ok(PlannedCiWork {
        requested_targets: requested_targets.to_vec(),
        packages: planned,
    })
}

fn assign_layers(
    app: &AppContext,
    packages: &mut BTreeMap<String, PlannedCiPackage>,
) -> Result<(), CoreError> {
    let mut indegree = BTreeMap::new();
    let mut dependents = BTreeMap::<String, Vec<String>>::new();

    for package_name in packages.keys() {
        let recipe = load_recipe(&app.database.layout().recipes_dir, package_name)?;
        let local_dependencies =
            local_dependency_targets(&app.database.layout().recipes_dir, &recipe.package.depends)?
                .into_iter()
                .chain(local_dependency_targets(
                    &app.database.layout().recipes_dir,
                    &recipe.package.makedepends,
                )?)
                .chain(local_dependency_targets(
                    &app.database.layout().recipes_dir,
                    &recipe.package.checkdepends,
                )?)
                .collect::<BTreeSet<_>>();

        indegree.insert(package_name.clone(), local_dependencies.len());
        for dependency in local_dependencies {
            dependents
                .entry(dependency)
                .or_default()
                .push(package_name.clone());
        }
    }

    let mut queue = indegree
        .iter()
        .filter_map(|(name, &count)| (count == 0).then_some(name.clone()))
        .collect::<VecDeque<_>>();
    let mut processed = 0_usize;

    while let Some(package_name) = queue.pop_front() {
        processed += 1;
        let package_layer = packages
            .get(&package_name)
            .map(|package| package.layer)
            .unwrap_or(0);
        for dependent in dependents.get(&package_name).into_iter().flatten() {
            if let Some(package) = packages.get_mut(dependent) {
                package.layer = package.layer.max(package_layer + 1);
            }
            if let Some(count) = indegree.get_mut(dependent) {
                *count -= 1;
                if *count == 0 {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    if processed == packages.len() {
        return Ok(());
    }

    Err(CoreError::Operator(
        "ci planning found a cycle in the local package closure".to_owned(),
    ))
}

fn local_dependency_targets(
    recipes_dir: &Path,
    entries: &[DependencyEntry],
) -> Result<Vec<String>, CoreError> {
    let mut resolved = Vec::new();
    for entry in entries {
        match &entry.body {
            DependencyBody::Constraint(value) => {
                let package_name = NamedConstraint::parse_dependency(value)
                    .map_err(|error| CoreError::Operator(error.to_string()))?
                    .name;
                if recipes_dir.join(&package_name).join("pkg.lua").is_file() {
                    resolved.push(package_name);
                }
            }
            DependencyBody::AnyOf(values) => {
                let selected = values.iter().find_map(|value| {
                    NamedConstraint::parse_dependency(value)
                        .ok()
                        .and_then(|constraint| {
                            recipes_dir
                                .join(&constraint.name)
                                .join("pkg.lua")
                                .is_file()
                                .then_some(constraint.name)
                        })
                });
                if let Some(package_name) = selected {
                    resolved.push(package_name);
                }
            }
        }
    }

    resolved.sort();
    resolved.dedup();
    Ok(resolved)
}

fn dependency_strings(entries: &[DependencyEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| {
            let body = match &entry.body {
                DependencyBody::Constraint(value) => value.clone(),
                DependencyBody::AnyOf(values) => format!("any({})", values.join(" | ")),
            };
            match entry.when.as_ref() {
                Some(predicate) => format!("{body} when [{}]", predicate.raw),
                None => body,
            }
        })
        .collect()
}
