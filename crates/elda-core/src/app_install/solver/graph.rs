use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::str::FromStr;

use pubgrub::Ranges;

use crate::CommandRequest;
use crate::app::{
    AppContext, DependencyCandidate, ParsedInstallRequest, ResolvedDependencyPlan,
    ResolvedInstallTarget,
};
use crate::app_install::parse_dependency_constraint;
use crate::app_install::provider_choice::{
    install_allows_provider_prompt, prompt_virtual_provider_selection, reorder_provider_candidates,
};
use crate::error::CoreError;
use elda_recipe::{DependencyBody, DependencyEntry};
use elda_types::{ConstraintVersion, PackageVersion};

use super::types::{SolverPackage, SolverVersion};

pub(crate) type SolverRange = Ranges<SolverVersion>;

#[derive(Debug, Clone)]
pub(crate) struct RootRequirement {
    pub(crate) target: String,
    pub(crate) package_name: String,
    pub(crate) package: SolverPackage,
    pub(crate) range: SolverRange,
}

#[derive(Debug, Clone)]
pub(crate) struct DependencyEdge {
    pub(crate) plan: ResolvedDependencyPlan,
    pub(crate) package: SolverPackage,
    pub(crate) range: SolverRange,
}

#[derive(Debug, Clone)]
pub(crate) struct RealPackageNode {
    pub(crate) package_name: String,
    pub(crate) resolved: ResolvedInstallTarget,
    pub(crate) version: PackageVersion,
    pub(crate) dependencies: Vec<DependencyEdge>,
    pub(crate) conflicts: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChoiceVersionNode {
    pub(crate) version: SolverVersion,
    pub(crate) package_name: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ChoicePackageNode {
    pub(crate) versions: Vec<ChoiceVersionNode>,
}

#[derive(Debug, Clone)]
pub(crate) struct SolverGraph {
    pub(crate) root_requirements: Vec<RootRequirement>,
    pub(crate) real_packages: BTreeMap<String, RealPackageNode>,
    pub(crate) choice_packages: BTreeMap<String, ChoicePackageNode>,
}

pub(crate) struct SolverGraphBuilder<'a> {
    app: &'a AppContext,
    request: &'a ParsedInstallRequest,
    command: Option<&'a CommandRequest>,
    provider_overrides: BTreeMap<String, String>,
    include_weak_for_packages: BTreeSet<String>,
    root_requirements: Vec<RootRequirement>,
    real_packages: BTreeMap<String, RealPackageNode>,
    choice_packages: BTreeMap<String, ChoicePackageNode>,
    expand_queue: VecDeque<String>,
    expanded: BTreeSet<String>,
    next_choice_id: usize,
}

impl<'a> SolverGraphBuilder<'a> {
    pub(crate) fn for_install(
        app: &'a AppContext,
        request: &'a ParsedInstallRequest,
        command: Option<&'a CommandRequest>,
    ) -> Result<SolverGraph, CoreError> {
        let mut builder = Self::new(app, request, command);

        for target in &request.targets {
            let resolved = app.resolve_install_target(target, request)?;
            let package_name = resolved.recipe.package.name.clone();
            builder
                .include_weak_for_packages
                .insert(package_name.clone());
            builder.insert_real_package(resolved)?;
            builder.root_requirements.push(RootRequirement {
                target: target.clone(),
                package_name: package_name.clone(),
                package: SolverPackage::Real(package_name.clone()),
                range: singleton_package_range(
                    builder
                        .real_packages
                        .get(&package_name)
                        .expect("root package should exist")
                        .version
                        .clone(),
                ),
            });
        }

        builder.expand_all()?;
        Ok(builder.finish())
    }

    pub(crate) fn for_upgrade(
        app: &'a AppContext,
        request: &'a ParsedInstallRequest,
        targets: &[String],
        refresh_weak_deps: bool,
        command: Option<&'a CommandRequest>,
    ) -> Result<SolverGraph, CoreError> {
        let mut builder = Self::new(app, request, command);

        for target in targets {
            let resolved = app.resolve_install_target(target, request)?;
            let package_name = resolved.recipe.package.name.clone();
            if refresh_weak_deps {
                builder
                    .include_weak_for_packages
                    .insert(package_name.clone());
            }
            builder.insert_real_package(resolved)?;
            builder.root_requirements.push(RootRequirement {
                target: target.clone(),
                package_name: package_name.clone(),
                package: SolverPackage::Real(package_name.clone()),
                range: singleton_package_range(
                    builder
                        .real_packages
                        .get(&package_name)
                        .expect("root package should exist")
                        .version
                        .clone(),
                ),
            });
        }

        builder.expand_all()?;
        Ok(builder.finish())
    }

    fn new(
        app: &'a AppContext,
        request: &'a ParsedInstallRequest,
        command: Option<&'a CommandRequest>,
    ) -> Self {
        Self {
            app,
            request,
            command,
            provider_overrides: BTreeMap::new(),
            include_weak_for_packages: BTreeSet::new(),
            root_requirements: Vec::new(),
            real_packages: BTreeMap::new(),
            choice_packages: BTreeMap::new(),
            expand_queue: VecDeque::new(),
            expanded: BTreeSet::new(),
            next_choice_id: 0,
        }
    }

    fn finish(self) -> SolverGraph {
        SolverGraph {
            root_requirements: self.root_requirements,
            real_packages: self.real_packages,
            choice_packages: self.choice_packages,
        }
    }

    fn expand_all(&mut self) -> Result<(), CoreError> {
        while let Some(package_name) = self.expand_queue.pop_front() {
            if self.expanded.contains(&package_name) {
                continue;
            }
            self.expand_real_package(&package_name)?;
            self.expanded.insert(package_name);
        }

        Ok(())
    }

    fn expand_real_package(&mut self, package_name: &str) -> Result<(), CoreError> {
        let (package, include_weak, effective_flags) = {
            let node = self
                .real_packages
                .get(package_name)
                .expect("real package should exist before expansion");
            (
                node.resolved.recipe.package.clone(),
                self.include_weak_for_packages.contains(package_name),
                node.resolved.flag_state.effective_flags.clone(),
            )
        };

        let active_depends = filter_dependency_entries(&package.depends, &effective_flags);
        let mut dependencies =
            self.resolve_dependency_entries("depends", false, &active_depends)?;
        if include_weak && self.app.config.defaults.install_recommends {
            let active_recommends =
                filter_dependency_entries(&package.recommends, &effective_flags);
            dependencies.extend(self.resolve_dependency_entries(
                "recommends",
                true,
                &active_recommends,
            )?);
        }
        let conflicts = self.resolve_conflict_packages(&package.conflicts)?;

        let node = self
            .real_packages
            .get_mut(package_name)
            .expect("expanded package should still exist");
        node.dependencies = dependencies;
        node.conflicts = conflicts;

        Ok(())
    }

    fn resolve_dependency_entries(
        &mut self,
        dependency_kind: &str,
        is_weak: bool,
        entries: &[DependencyEntry],
    ) -> Result<Vec<DependencyEdge>, CoreError> {
        let mut dependencies = Vec::with_capacity(entries.len());

        for entry in entries {
            match &entry.body {
                DependencyBody::Constraint(value) => dependencies
                    .push(self.resolve_named_dependency(dependency_kind, is_weak, value)?),
                DependencyBody::AnyOf(values) => dependencies
                    .push(self.resolve_any_of_dependency(dependency_kind, is_weak, values)?),
            }
        }

        Ok(dependencies)
    }

    fn resolve_named_dependency(
        &mut self,
        dependency_kind: &str,
        is_weak: bool,
        value: &str,
    ) -> Result<DependencyEdge, CoreError> {
        let constraint = parse_dependency_constraint(value)?;
        let plan = ResolvedDependencyPlan {
            target: String::new(),
            dependency_name: constraint.name.clone(),
            dependency_kind: dependency_kind.to_owned(),
            raw_expr: value.to_owned(),
            is_weak,
            provider_group: None,
        };

        if let Some(package_name) = self.try_resolve_exact_package_name(&constraint.name)? {
            let version = self
                .real_packages
                .get(&package_name)
                .expect("exact package should be registered")
                .version
                .clone();
            let matches = constraint.matches_version(&ConstraintVersion::from(&version));

            if is_weak {
                let exact_packages = if matches {
                    vec![package_name]
                } else {
                    Vec::new()
                };
                return self.wrap_choice_edge(plan, exact_packages, true, &constraint.name);
            }

            return Ok(DependencyEdge {
                plan,
                package: SolverPackage::Real(package_name),
                range: if matches {
                    singleton_package_range(version)
                } else {
                    SolverRange::empty()
                },
            });
        }

        let packages = self.provider_package_names(&constraint.name, value)?;
        self.wrap_choice_edge(plan, packages, is_weak, &constraint.name)
    }

    fn resolve_any_of_dependency(
        &mut self,
        dependency_kind: &str,
        is_weak: bool,
        values: &[String],
    ) -> Result<DependencyEdge, CoreError> {
        let mut exact_packages = Vec::new();
        for value in values {
            let constraint = parse_dependency_constraint(value)?;
            let Some(package_name) = self.try_resolve_exact_package_name(&constraint.name)? else {
                continue;
            };
            let version = self
                .real_packages
                .get(&package_name)
                .expect("exact package should be registered")
                .version
                .clone();
            if constraint.matches_version(&ConstraintVersion::from(&version))
                && !exact_packages.contains(&package_name)
            {
                exact_packages.push(package_name);
            }
        }

        let plan = ResolvedDependencyPlan {
            target: String::new(),
            dependency_name: values.first().cloned().unwrap_or_default(),
            dependency_kind: dependency_kind.to_owned(),
            raw_expr: format!("any({})", values.join(" | ")),
            is_weak,
            provider_group: Some(values.join(" | ")),
        };
        let label_source = values.first().cloned().unwrap_or_default();
        if !exact_packages.is_empty() {
            return self.wrap_choice_edge(plan, exact_packages, is_weak, &label_source);
        }

        let mut provider_packages = Vec::new();
        for value in values {
            let constraint = parse_dependency_constraint(value)?;
            for package_name in self.provider_package_names(&constraint.name, value)? {
                if !provider_packages.contains(&package_name) {
                    provider_packages.push(package_name);
                }
            }
        }

        self.wrap_choice_edge(plan, provider_packages, is_weak, &label_source)
    }

    fn wrap_choice_edge(
        &mut self,
        plan: ResolvedDependencyPlan,
        package_names: Vec<String>,
        allow_absent: bool,
        label_hint: &str,
    ) -> Result<DependencyEdge, CoreError> {
        let package = self.register_choice_package(package_names, allow_absent, label_hint);

        Ok(DependencyEdge {
            plan,
            package,
            range: SolverRange::full(),
        })
    }

    fn register_choice_package(
        &mut self,
        package_names: Vec<String>,
        allow_absent: bool,
        label_hint: &str,
    ) -> SolverPackage {
        self.next_choice_id += 1;
        let label = sanitize_choice_label(label_hint);
        let key = if label.is_empty() {
            format!("choice-{}", self.next_choice_id)
        } else {
            format!("choice-{}:{label}", self.next_choice_id)
        };
        let mut versions = Vec::new();
        if allow_absent {
            versions.push(ChoiceVersionNode {
                version: SolverVersion::Absent,
                package_name: None,
            });
        }
        let choice_count = package_names.len() as u32;
        for (index, package_name) in package_names.into_iter().enumerate() {
            let rank = choice_count.saturating_sub(index as u32);
            versions.push(ChoiceVersionNode {
                version: SolverVersion::Choice(rank),
                package_name: Some(package_name),
            });
        }

        self.choice_packages
            .insert(key.clone(), ChoicePackageNode { versions });

        SolverPackage::Choice(key)
    }

    fn provider_package_names(
        &mut self,
        virtual_name: &str,
        dependency_expr: &str,
    ) -> Result<Vec<String>, CoreError> {
        let candidates = self
            .app
            .provider_candidates(&parse_dependency_constraint(dependency_expr)?, self.request)?;
        let ranked = self.rank_provider_candidates(virtual_name, &candidates)?;
        let mut packages = Vec::new();

        for candidate in ranked {
            let package_name = candidate.target;
            self.ensure_real_package_by_name(&package_name)?;
            if !packages.contains(&package_name) {
                packages.push(package_name);
            }
        }

        Ok(packages)
    }

    fn rank_provider_candidates(
        &mut self,
        virtual_name: &str,
        candidates: &[DependencyCandidate],
    ) -> Result<Vec<DependencyCandidate>, CoreError> {
        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let preferences = self
            .app
            .config
            .resolver
            .provider_preferences
            .get(virtual_name);
        let mut ranked = candidates.to_vec();
        ranked.sort_by(|left, right| {
            provider_preference_rank(preferences, &left.target)
                .cmp(&provider_preference_rank(preferences, &right.target))
                .then_with(|| left.source_priority.cmp(&right.source_priority))
                .then_with(|| right.candidate_version.cmp(&left.candidate_version))
                .then_with(|| left.target.cmp(&right.target))
        });

        if let Some(chosen) = self
            .request
            .provider_choices
            .get(virtual_name)
            .or_else(|| self.provider_overrides.get(virtual_name))
        {
            reorder_provider_candidates(&mut ranked, chosen)?;
            return Ok(ranked);
        }

        if self.provider_candidates_are_ambiguous(preferences, &ranked) {
            if install_allows_provider_prompt(self.command) {
                let chosen = prompt_virtual_provider_selection(virtual_name, &ranked)?;
                self.provider_overrides
                    .insert(virtual_name.to_owned(), chosen.clone());
                reorder_provider_candidates(&mut ranked, &chosen)?;
                return Ok(ranked);
            }
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!(
                    "ambiguous virtual provider for `{virtual_name}`; candidates are `{}`; pass `--provider {virtual_name}=<package>` or set [resolver.provider_preferences].{virtual_name}",
                    ranked
                        .iter()
                        .map(|candidate| candidate.target.as_str())
                        .collect::<Vec<_>>()
                        .join("`, `")
                ),
            )));
        }

        Ok(ranked)
    }

    fn provider_candidates_are_ambiguous(
        &self,
        preferences: Option<&Vec<String>>,
        candidates: &[DependencyCandidate],
    ) -> bool {
        if candidates.len() < 2 {
            return false;
        }

        let best = &candidates[0];
        let tied = candidates
            .iter()
            .take_while(|candidate| {
                provider_preference_rank(preferences, &candidate.target)
                    == provider_preference_rank(preferences, &best.target)
                    && candidate.source_priority == best.source_priority
                    && candidate.candidate_version == best.candidate_version
            })
            .count();
        if tied < 2 {
            return false;
        }

        let best_is_local = best.source_priority.is_none();
        let tied_candidates = &candidates[..tied];
        let mixed_origin = tied_candidates
            .iter()
            .any(|candidate| candidate.source_priority.is_none() != best_is_local);
        if mixed_origin {
            return true;
        }

        if best_is_local {
            return true;
        }

        true
    }

    fn resolve_conflict_packages(
        &mut self,
        conflicts: &[String],
    ) -> Result<Vec<String>, CoreError> {
        let mut packages = Vec::new();

        for conflict in conflicts {
            let constraint = parse_dependency_constraint(conflict)?;
            let Some(package_name) = self.try_resolve_exact_package_name(&constraint.name)? else {
                continue;
            };
            if !packages.contains(&package_name) {
                packages.push(package_name);
            }
        }

        Ok(packages)
    }

    fn try_resolve_exact_package_name(
        &mut self,
        package_name: &str,
    ) -> Result<Option<String>, CoreError> {
        if self.real_packages.contains_key(package_name) {
            return Ok(Some(package_name.to_owned()));
        }

        self.ensure_real_package_by_name(package_name)
    }

    fn ensure_real_package_by_name(
        &mut self,
        package_name: &str,
    ) -> Result<Option<String>, CoreError> {
        let resolved = match self.app.resolve_install_target(package_name, self.request) {
            Ok(resolved) => resolved,
            Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(message)))
                if message.starts_with("no local recipe or synced package named") =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let name = resolved.recipe.package.name.clone();
        self.insert_real_package(resolved)?;

        Ok(Some(name))
    }

    fn insert_real_package(&mut self, resolved: ResolvedInstallTarget) -> Result<(), CoreError> {
        let package_name = resolved.recipe.package.name.clone();
        if self.real_packages.contains_key(&package_name) {
            return Ok(());
        }

        let version = PackageVersion::from_str(&format!(
            "{}:{}-{}",
            resolved.recipe.package.epoch,
            resolved.recipe.package.version,
            resolved.recipe.package.rel,
        ))
        .map_err(|error| CoreError::Operator(error.to_string()))?;

        self.real_packages.insert(
            package_name.clone(),
            RealPackageNode {
                package_name: package_name.clone(),
                resolved,
                version,
                dependencies: Vec::new(),
                conflicts: Vec::new(),
            },
        );
        self.expand_queue.push_back(package_name);

        Ok(())
    }
}

fn provider_preference_rank(preferences: Option<&Vec<String>>, package_name: &str) -> usize {
    preferences
        .and_then(|values| values.iter().position(|value| value == package_name))
        .unwrap_or(usize::MAX)
}

pub(crate) fn singleton_package_range(version: PackageVersion) -> SolverRange {
    SolverRange::singleton(SolverVersion::Package(version))
}

pub(crate) fn singleton_absent_range() -> SolverRange {
    SolverRange::singleton(SolverVersion::Absent)
}

fn sanitize_choice_label(label: &str) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut sanitized = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric()
            || matches!(
                ch,
                '-' | '_' | '.' | ':' | '+' | '|' | '/' | '<' | '>' | '=' | '!'
            )
        {
            sanitized.push(ch);
        } else if ch.is_whitespace() {
            sanitized.push('_');
        } else {
            sanitized.push('?');
        }
    }
    if sanitized.len() > 64 {
        sanitized.truncate(64);
    }
    sanitized
}

pub(crate) fn filter_dependency_entries(
    entries: &[DependencyEntry],
    effective_flags: &BTreeMap<String, bool>,
) -> Vec<DependencyEntry> {
    entries
        .iter()
        .filter(|entry| {
            entry
                .when
                .as_ref()
                .is_none_or(|predicate| predicate.evaluate(effective_flags))
        })
        .cloned()
        .collect()
}
