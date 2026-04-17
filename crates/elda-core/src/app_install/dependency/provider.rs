use std::collections::BTreeMap;
use std::fs;
use std::str::FromStr;

use crate::app::{AppContext, DependencyCandidate, ParsedInstallRequest};
use crate::app_install::dependency::constraint::{
    installed_package_satisfies_constraint, package_satisfies_constraint,
    provides_satisfy_constraint,
};
use crate::error::CoreError;
use elda_recipe::load_recipe;
use elda_repo::{RepoError, load_snapshot};
use elda_types::{NamedConstraint, PackageVersion};

#[derive(Debug, Clone)]
struct ProviderPackageRecord {
    package_name: String,
    source_priority: Option<u32>,
    candidate_version: PackageVersion,
}

impl AppContext {
    pub(crate) fn exact_dependency_candidate(
        &self,
        constraint: &NamedConstraint,
        request: &ParsedInstallRequest,
    ) -> Result<Option<DependencyCandidate>, CoreError> {
        let installed = self
            .database
            .installed_package(&constraint.name)?
            .is_some_and(|package| installed_package_satisfies_constraint(&package, constraint));
        if installed {
            return Ok(Some(DependencyCandidate {
                target: constraint.name.clone(),
                installed: true,
                source_priority: None,
                candidate_version: self.database.installed_package(&constraint.name)?.map(
                    |package| PackageVersion {
                        epoch: package.epoch,
                        pkgver: package.pkgver,
                        pkgrel: package.pkgrel,
                    },
                ),
            }));
        }

        let available = self
            .resolve_install_target(&constraint.name, request)
            .map(|resolved| package_satisfies_constraint(&resolved.recipe.package, constraint))
            .unwrap_or(false);
        if available {
            return Ok(Some(DependencyCandidate {
                target: constraint.name.clone(),
                installed: false,
                source_priority: None,
                candidate_version: None,
            }));
        }

        Ok(None)
    }

    pub(crate) fn provider_candidates(
        &self,
        constraint: &NamedConstraint,
        _request: &ParsedInstallRequest,
    ) -> Result<Vec<DependencyCandidate>, CoreError> {
        let mut candidates = BTreeMap::<String, DependencyCandidate>::new();

        self.add_provider_candidates(
            &mut candidates,
            self.local_provider_packages(constraint)?,
            constraint,
        )?;
        self.add_provider_candidates(
            &mut candidates,
            self.synced_provider_packages(constraint)?,
            constraint,
        )?;

        Ok(candidates.into_values().collect())
    }

    fn local_provider_packages(
        &self,
        constraint: &NamedConstraint,
    ) -> Result<Vec<ProviderPackageRecord>, CoreError> {
        let recipes_dir = &self.database.layout().recipes_dir;
        if !recipes_dir.exists() {
            return Ok(Vec::new());
        }

        let mut packages = Vec::new();
        for entry in fs::read_dir(recipes_dir)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            let Some(package_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !path.join("pkg.lua").is_file() {
                continue;
            }
            let recipe = load_recipe(recipes_dir, package_name)?;
            if provides_satisfy_constraint(&recipe.package.provides, constraint)? {
                packages.push(ProviderPackageRecord {
                    package_name: recipe.package.name,
                    source_priority: None,
                    candidate_version: PackageVersion {
                        epoch: recipe.package.epoch,
                        pkgver: recipe.package.version,
                        pkgrel: recipe.package.rel,
                    },
                });
            }
        }

        Ok(sorted_unique_records(packages))
    }

    fn synced_provider_packages(
        &self,
        constraint: &NamedConstraint,
    ) -> Result<Vec<ProviderPackageRecord>, CoreError> {
        let snapshot = match load_snapshot(&self.repo_snapshot_path()) {
            Ok(snapshot) => snapshot,
            Err(RepoError::SnapshotMissing) => return Ok(Vec::new()),
            Err(error) => return Err(CoreError::Repo(error)),
        };

        let mut packages = Vec::new();
        for package in snapshot.packages {
            let recipe = package.parse_recipe()?;
            if provides_satisfy_constraint(&recipe.package.provides, constraint)? {
                packages.push(ProviderPackageRecord {
                    package_name: recipe.package.name,
                    source_priority: Some(package.remote_priority),
                    candidate_version: PackageVersion::from_str(&package.version_string())
                        .map_err(|error| {
                            CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                                error.to_string(),
                            ))
                        })?,
                });
            }
        }

        Ok(sorted_unique_records(packages))
    }

    fn add_provider_candidates(
        &self,
        candidates: &mut BTreeMap<String, DependencyCandidate>,
        packages: Vec<ProviderPackageRecord>,
        constraint: &NamedConstraint,
    ) -> Result<(), CoreError> {
        for package in packages {
            let installed = self
                .database
                .installed_package(&package.package_name)?
                .is_some_and(|installed_package| {
                    installed_package_satisfies_constraint(&installed_package, constraint)
                });
            if installed {
                candidates.insert(
                    package.package_name.clone(),
                    DependencyCandidate {
                        target: package.package_name,
                        installed: true,
                        source_priority: package.source_priority,
                        candidate_version: Some(package.candidate_version),
                    },
                );
                continue;
            }

            candidates.insert(
                package.package_name.clone(),
                DependencyCandidate {
                    target: package.package_name,
                    installed: false,
                    source_priority: package.source_priority,
                    candidate_version: Some(package.candidate_version),
                },
            );
        }

        Ok(())
    }
}

fn sorted_unique_records(mut packages: Vec<ProviderPackageRecord>) -> Vec<ProviderPackageRecord> {
    packages.sort_by(|left, right| left.package_name.cmp(&right.package_name));
    packages.dedup_by(|left, right| left.package_name == right.package_name);
    packages
}
