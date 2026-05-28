mod graph;
mod provider;
mod types;

use std::collections::{BTreeMap, BTreeSet};

use pubgrub::{DefaultStringReporter, PubGrubError, Reporter, SelectedDependencies, resolve};

use crate::CommandRequest;
use crate::app::{AppContext, ParsedInstallRequest, ResolvedDependencyPlan, ResolvedInstallTarget};
use crate::error::CoreError;

use self::graph::{RealPackageNode, SolverGraph, SolverGraphBuilder};
use self::types::{SolverPackage, SolverVersion};

#[derive(Debug, Clone)]
pub(crate) struct SolvedPackage {
    pub(crate) package_name: String,
    pub(crate) resolved: ResolvedInstallTarget,
    pub(crate) dependencies: Vec<ResolvedDependencyPlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct SolverResolution {
    pub(crate) explicit_targets: BTreeMap<String, String>,
    pub(crate) packages: BTreeMap<String, SolvedPackage>,
    pub(crate) order: Vec<String>,
}

impl AppContext {
    pub(crate) fn solve_install_request(
        &self,
        request: &ParsedInstallRequest,
        command: Option<&CommandRequest>,
    ) -> Result<SolverResolution, CoreError> {
        let graph = SolverGraphBuilder::for_install(self, request, command)?;
        solve_graph(graph)
    }

    pub(crate) fn solve_upgrade_request(
        &self,
        request: &ParsedInstallRequest,
        targets: &[String],
        refresh_weak_deps: bool,
        command: Option<&CommandRequest>,
    ) -> Result<SolverResolution, CoreError> {
        let graph =
            SolverGraphBuilder::for_upgrade(self, request, targets, refresh_weak_deps, command)?;
        solve_graph(graph)
    }
}

fn solve_graph(graph: SolverGraph) -> Result<SolverResolution, CoreError> {
    let selected =
        resolve(&graph, SolverPackage::Root, SolverVersion::Root).map_err(pubgrub_error_to_core)?;
    let explicit_targets = graph
        .root_requirements
        .iter()
        .map(|requirement| (requirement.target.clone(), requirement.package_name.clone()))
        .collect::<BTreeMap<_, _>>();

    let packages = graph
        .real_packages
        .iter()
        .filter(|(package_name, node)| is_selected_real_package(&selected, package_name, node))
        .map(|(package_name, node)| {
            resolve_package(node, &graph, &selected).map(|package| (package_name.clone(), package))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let order = package_order(&graph, &packages);

    Ok(SolverResolution {
        explicit_targets,
        packages,
        order,
    })
}

fn is_selected_real_package(
    selected: &SelectedDependencies<SolverPackage, SolverVersion>,
    package_name: &str,
    node: &RealPackageNode,
) -> bool {
    selected.get(&SolverPackage::Real(package_name.to_owned()))
        == Some(&SolverVersion::Package(node.version.clone()))
}

fn resolve_package(
    node: &RealPackageNode,
    graph: &SolverGraph,
    selected: &SelectedDependencies<SolverPackage, SolverVersion>,
) -> Result<SolvedPackage, CoreError> {
    let mut dependencies = Vec::new();

    for dependency in &node.dependencies {
        let Some(target) = resolve_dependency_target(graph, selected, &dependency.package) else {
            continue;
        };
        dependencies.push(ResolvedDependencyPlan {
            target: target.clone(),
            dependency_name: target,
            dependency_kind: dependency.plan.dependency_kind.clone(),
            raw_expr: dependency.plan.raw_expr.clone(),
            is_weak: dependency.plan.is_weak,
            provider_group: dependency.plan.provider_group.clone(),
        });
    }

    Ok(SolvedPackage {
        package_name: node.package_name.clone(),
        resolved: node.resolved.clone(),
        dependencies,
    })
}

fn resolve_dependency_target(
    graph: &SolverGraph,
    selected: &SelectedDependencies<SolverPackage, SolverVersion>,
    package: &SolverPackage,
) -> Option<String> {
    match package {
        SolverPackage::Real(package_name) => selected
            .get(&SolverPackage::Real(package_name.clone()))
            .filter(|version| matches!(version, SolverVersion::Package(_)))
            .map(|_| package_name.clone()),
        SolverPackage::Choice(key) => {
            let selected_version = selected.get(&SolverPackage::Choice(key.clone()))?;
            let choice = graph.choice_packages.get(key)?;
            choice
                .versions
                .iter()
                .find(|candidate| &candidate.version == selected_version)
                .and_then(|candidate| candidate.package_name.clone())
        }
        SolverPackage::Root => None,
    }
}

fn package_order(graph: &SolverGraph, packages: &BTreeMap<String, SolvedPackage>) -> Vec<String> {
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();

    for requirement in &graph.root_requirements {
        visit_package(
            &requirement.package_name,
            packages,
            &mut visited,
            &mut ordered,
        );
    }

    ordered
}

fn visit_package(
    package_name: &str,
    packages: &BTreeMap<String, SolvedPackage>,
    visited: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) {
    if !visited.insert(package_name.to_owned()) {
        return;
    }
    let Some(package) = packages.get(package_name) else {
        return;
    };

    for dependency in &package.dependencies {
        visit_package(&dependency.target, packages, visited, ordered);
    }
    ordered.push(package_name.to_owned());
}

fn pubgrub_error_to_core(error: PubGrubError<SolverGraph>) -> CoreError {
    match error {
        PubGrubError::NoSolution(mut derivation_tree) => {
            derivation_tree.collapse_no_versions();
            CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(format!(
                "dependency resolution failed: {}",
                DefaultStringReporter::report(&derivation_tree)
            )))
        }
        other => CoreError::Operator(format!("dependency resolution failed: {other}")),
    }
}
