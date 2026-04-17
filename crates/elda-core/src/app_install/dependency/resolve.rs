use std::collections::BTreeMap;

use crate::app::{AppContext, DependencyCandidate, ParsedInstallRequest, ResolvedDependencyPlan};
use crate::app_install::dependency::constraint::parse_dependency_constraint;
use crate::error::CoreError;
use elda_recipe::DependencyEntry;

impl AppContext {
    pub(crate) fn collect_install_dependencies(
        &self,
        package: &elda_recipe::PackageDefinition,
        install_reason: &str,
        request: &ParsedInstallRequest,
    ) -> Result<Vec<ResolvedDependencyPlan>, CoreError> {
        let mut dependencies = self.collect_required_dependencies(package, request)?;
        if install_reason == "explicit" && self.config.defaults.install_recommends {
            dependencies.extend(self.resolve_dependency_family(
                "recommends",
                true,
                &package.recommends,
                request,
            )?);
        }

        Ok(dependencies)
    }

    pub(crate) fn collect_required_dependencies(
        &self,
        package: &elda_recipe::PackageDefinition,
        request: &ParsedInstallRequest,
    ) -> Result<Vec<ResolvedDependencyPlan>, CoreError> {
        self.resolve_dependency_family("depends", false, &package.depends, request)
    }

    pub(crate) fn resolve_dependency_family(
        &self,
        dependency_kind: &str,
        is_weak: bool,
        entries: &[DependencyEntry],
        request: &ParsedInstallRequest,
    ) -> Result<Vec<ResolvedDependencyPlan>, CoreError> {
        let mut dependencies = Vec::with_capacity(entries.len());

        for entry in entries {
            match entry {
                DependencyEntry::Constraint(value) => {
                    dependencies.push(self.named_dependency_plan(
                        dependency_kind,
                        is_weak,
                        value,
                        request,
                    )?);
                }
                DependencyEntry::AnyOf(providers) => {
                    dependencies.push(self.any_of_dependency_plan(
                        dependency_kind,
                        is_weak,
                        providers,
                        request,
                    )?);
                }
            }
        }

        Ok(dependencies)
    }

    pub(crate) fn resolve_named_dependency_candidate(
        &self,
        dependency_expr: &str,
        request: &ParsedInstallRequest,
    ) -> Result<DependencyCandidate, CoreError> {
        let constraint = parse_dependency_constraint(dependency_expr)?;
        if let Some(candidate) = self.exact_dependency_candidate(&constraint, request)? {
            return Ok(candidate);
        }

        let providers = self.provider_candidates(&constraint, request)?;
        if providers.is_empty() {
            let message = if constraint.is_versioned() {
                format!("no package or explicit versioned provide satisfies `{dependency_expr}`")
            } else {
                format!("no package or virtual provider satisfies `{dependency_expr}`")
            };
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                message,
            )));
        }
        self.select_unique_dependency_candidate(
            &providers,
            &format!("virtual provider for `{dependency_expr}`"),
        )
    }

    pub(crate) fn resolve_any_of_dependency_candidate(
        &self,
        providers: &[String],
        request: &ParsedInstallRequest,
    ) -> Result<DependencyCandidate, CoreError> {
        let exact_candidates = self.any_of_exact_candidates(providers, request)?;
        let provider_candidates = self.any_of_provider_candidates(providers, request)?;
        let context = format!("dependency alternatives `{}`", providers.join(" | "));

        if let Some(candidate) =
            Self::select_preferred_dependency_candidate(exact_candidates, &context)?
        {
            return Ok(candidate);
        }
        if let Some(candidate) =
            Self::select_preferred_dependency_candidate(provider_candidates, &context)?
        {
            return Ok(candidate);
        }

        Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
            format!(
                "no available provider satisfies dependency alternatives `{}`",
                providers.join(" | ")
            ),
        )))
    }

    fn named_dependency_plan(
        &self,
        dependency_kind: &str,
        is_weak: bool,
        value: &str,
        request: &ParsedInstallRequest,
    ) -> Result<ResolvedDependencyPlan, CoreError> {
        let candidate = self.resolve_named_dependency_candidate(value, request)?;

        Ok(ResolvedDependencyPlan {
            target: candidate.target.clone(),
            dependency_name: candidate.target,
            dependency_kind: dependency_kind.to_owned(),
            raw_expr: value.to_owned(),
            is_weak,
            provider_group: None,
        })
    }

    fn any_of_dependency_plan(
        &self,
        dependency_kind: &str,
        is_weak: bool,
        providers: &[String],
        request: &ParsedInstallRequest,
    ) -> Result<ResolvedDependencyPlan, CoreError> {
        let provider_group = providers.join(" | ");
        let candidate = self.resolve_any_of_dependency_candidate(providers, request)?;

        Ok(ResolvedDependencyPlan {
            target: candidate.target.clone(),
            dependency_name: candidate.target,
            dependency_kind: dependency_kind.to_owned(),
            raw_expr: format!("any({provider_group})"),
            is_weak,
            provider_group: Some(provider_group),
        })
    }

    fn any_of_exact_candidates(
        &self,
        providers: &[String],
        request: &ParsedInstallRequest,
    ) -> Result<Vec<DependencyCandidate>, CoreError> {
        let mut exact_candidates = BTreeMap::<String, DependencyCandidate>::new();

        for provider in providers {
            let constraint = parse_dependency_constraint(provider)?;
            if let Some(candidate) = self.exact_dependency_candidate(&constraint, request)? {
                exact_candidates.insert(candidate.target.clone(), candidate);
            }
        }

        Ok(exact_candidates.into_values().collect())
    }

    fn any_of_provider_candidates(
        &self,
        providers: &[String],
        request: &ParsedInstallRequest,
    ) -> Result<Vec<DependencyCandidate>, CoreError> {
        let mut provider_candidates = BTreeMap::<String, DependencyCandidate>::new();

        for provider in providers {
            let constraint = parse_dependency_constraint(provider)?;
            for candidate in self.provider_candidates(&constraint, request)? {
                provider_candidates.insert(candidate.target.clone(), candidate);
            }
        }

        Ok(provider_candidates.into_values().collect())
    }
}
