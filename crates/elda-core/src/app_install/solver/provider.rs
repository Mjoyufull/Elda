use std::cmp::Reverse;

use pubgrub::{Dependencies, DependencyProvider, PackageResolutionStatistics};

use super::graph::{SolverGraph, singleton_absent_range, singleton_package_range};
use super::types::{SolverPackage, SolverVersion};

impl DependencyProvider for SolverGraph {
    type P = SolverPackage;
    type V = SolverVersion;
    type VS = super::graph::SolverRange;
    type M = String;
    type Err = std::convert::Infallible;
    type Priority = (u32, Reverse<usize>);

    fn choose_version(
        &self,
        package: &Self::P,
        range: &Self::VS,
    ) -> Result<Option<Self::V>, Self::Err> {
        let candidates = match package {
            SolverPackage::Root => vec![SolverVersion::Root],
            SolverPackage::Real(package_name) => self
                .real_packages
                .get(package_name)
                .map(|node| {
                    vec![
                        SolverVersion::Absent,
                        SolverVersion::Package(node.version.clone()),
                    ]
                })
                .unwrap_or_default(),
            SolverPackage::Choice(key) => self
                .choice_packages
                .get(key)
                .map(|node| {
                    node.versions
                        .iter()
                        .map(|version| version.version.clone())
                        .collect()
                })
                .unwrap_or_default(),
        };

        Ok(candidates
            .into_iter()
            .filter(|candidate| range.contains(candidate))
            .max())
    }

    fn prioritize(
        &self,
        package: &Self::P,
        range: &Self::VS,
        package_statistics: &PackageResolutionStatistics,
    ) -> Self::Priority {
        let version_count = match package {
            SolverPackage::Root => 1,
            SolverPackage::Real(package_name) => self
                .real_packages
                .get(package_name)
                .map(|node| {
                    [
                        SolverVersion::Absent,
                        SolverVersion::Package(node.version.clone()),
                    ]
                    .into_iter()
                    .filter(|version| range.contains(version))
                    .count()
                })
                .unwrap_or_default(),
            SolverPackage::Choice(key) => self
                .choice_packages
                .get(key)
                .map(|node| {
                    node.versions
                        .iter()
                        .filter(|version| range.contains(&version.version))
                        .count()
                })
                .unwrap_or_default(),
        };

        if version_count == 0 {
            return (u32::MAX, Reverse(0));
        }

        (package_statistics.conflict_count(), Reverse(version_count))
    }

    fn get_dependencies(
        &self,
        package: &Self::P,
        version: &Self::V,
    ) -> Result<Dependencies<Self::P, Self::VS, Self::M>, Self::Err> {
        let dependencies = match package {
            SolverPackage::Root => {
                if version != &SolverVersion::Root {
                    return Ok(Dependencies::Unavailable(
                        "unexpected root version".to_owned(),
                    ));
                }
                self.root_dependency_constraints()
            }
            SolverPackage::Real(package_name) => {
                self.real_package_constraints(package_name, version)
            }
            SolverPackage::Choice(key) => self.choice_package_constraints(key, version),
        };

        Ok(dependencies)
    }
}

impl SolverGraph {
    fn root_dependency_constraints(
        &self,
    ) -> Dependencies<SolverPackage, super::graph::SolverRange, String> {
        Dependencies::Available(
            self.root_requirements
                .iter()
                .map(|requirement| (requirement.package.clone(), requirement.range.clone()))
                .collect(),
        )
    }

    fn real_package_constraints(
        &self,
        package_name: &str,
        version: &SolverVersion,
    ) -> Dependencies<SolverPackage, super::graph::SolverRange, String> {
        let Some(node) = self.real_packages.get(package_name) else {
            return Dependencies::Unavailable(
                "package is not present in the solver graph".to_owned(),
            );
        };
        if version == &SolverVersion::Absent {
            return Dependencies::Available(Vec::new().into_iter().collect());
        }
        if version != &SolverVersion::Package(node.version.clone()) {
            return Dependencies::Unavailable("selected version does not exist".to_owned());
        }

        let mut constraints = node
            .dependencies
            .iter()
            .map(|dependency| (dependency.package.clone(), dependency.range.clone()))
            .collect::<Vec<_>>();
        for conflict in &node.conflicts {
            if self.real_packages.contains_key(conflict) {
                constraints.push((
                    SolverPackage::Real(conflict.clone()),
                    singleton_absent_range(),
                ));
            }
        }

        Dependencies::Available(constraints.into_iter().collect())
    }

    fn choice_package_constraints(
        &self,
        key: &str,
        version: &SolverVersion,
    ) -> Dependencies<SolverPackage, super::graph::SolverRange, String> {
        let Some(node) = self.choice_packages.get(key) else {
            return Dependencies::Unavailable("choice package is missing".to_owned());
        };
        let Some(selected) = node
            .versions
            .iter()
            .find(|candidate| &candidate.version == version)
        else {
            return Dependencies::Unavailable("selected choice version does not exist".to_owned());
        };
        let Some(package_name) = &selected.package_name else {
            return Dependencies::Available(Vec::new().into_iter().collect());
        };
        let Some(real_package) = self.real_packages.get(package_name) else {
            return Dependencies::Unavailable("choice target package is missing".to_owned());
        };

        Dependencies::Available(
            [(
                SolverPackage::Real(package_name.clone()),
                singleton_package_range(real_package.version.clone()),
            )]
            .into_iter()
            .collect(),
        )
    }
}
