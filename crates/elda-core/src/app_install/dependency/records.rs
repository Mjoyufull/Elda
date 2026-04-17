use crate::app::{AppContext, ResolvedDependencyPlan};
use elda_build::PackageDependency;

impl AppContext {
    pub(crate) fn planned_dependency_records(
        dependencies: &[ResolvedDependencyPlan],
    ) -> Vec<PackageDependency> {
        dependencies
            .iter()
            .map(|dependency| PackageDependency {
                dependency_name: dependency.dependency_name.clone(),
                dependency_kind: dependency.dependency_kind.clone(),
                raw_expr: dependency.raw_expr.clone(),
                is_weak: dependency.is_weak,
                provider_group: dependency.provider_group.clone(),
            })
            .collect()
    }
}
