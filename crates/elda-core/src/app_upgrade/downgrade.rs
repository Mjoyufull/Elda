use super::*;

use elda_install::{DowngradeCandidate, downgrade_to_candidate, list_downgrade_candidates};
use elda_types::{ConstraintVersion, NamedConstraint};

impl AppContext {
    pub(crate) fn handle_downgrade(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = self.parse_downgrade_request(&request)?;
        let installed = self.ensure_installed(&parsed.package)?;
        let current_version = PackageVersion::from_str(&installed_version(&installed))
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        let candidate = self.select_downgrade_candidate(&installed, &current_version, &parsed)?;
        self.validate_downgrade_policy(&installed, &candidate)?;
        self.validate_downgrade_reverse_dependencies(&installed.pkgname, &candidate)?;

        if request.dry_run {
            return Ok(CommandReport {
                area: "plan",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: format!(
                    "planned downgrade of `{}` from {} to {}.",
                    installed.pkgname,
                    installed_version(&installed),
                    candidate.version(),
                ),
                details: Some(json!({
                    "plan": {
                        "kind": "downgrade",
                        "package": installed.pkgname,
                        "installed_version": installed_version(&installed),
                        "candidate": candidate,
                    },
                })),
            });
        }

        let report = downgrade_to_candidate(
            &self.database,
            &candidate,
            &installed.install_reason,
            installed.pinned_version.clone(),
            installed.held,
            installed.hold_source.clone(),
            &self.mutation_policy(),
        )?;

        Ok(CommandReport {
            area: "downgrade",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "downgraded `{}` from {} to {}.",
                installed.pkgname,
                installed_version(&installed),
                candidate.version(),
            ),
            details: Some(json!({
                "package": installed.pkgname,
                "installed_version": installed_version(&installed),
                "candidate": candidate,
                "install": report,
            })),
        })
    }

    fn select_downgrade_candidate(
        &self,
        installed: &elda_db::InstalledPackageDetails,
        current_version: &PackageVersion,
        parsed: &crate::app::ParsedDowngradeRequest,
    ) -> Result<DowngradeCandidate, CoreError> {
        let installed_arch = installed.arch.as_deref().ok_or_else(|| {
            CoreError::Operator(format!(
                "installed package `{}` is missing its canonical arch",
                installed.pkgname
            ))
        })?;
        let candidates = list_downgrade_candidates(&self.database, &installed.pkgname)?;
        let requested = parsed.version.as_ref();

        candidates
            .into_iter()
            .filter(|candidate| candidate.arch == installed_arch)
            .find(|candidate| {
                let candidate_version = candidate.version();
                candidate_version < *current_version
                    && requested.is_none_or(|requested| requested == &candidate_version)
            })
            .ok_or_else(|| {
                let requested = requested
                    .map(|version| format!(" matching requested version {version}"))
                    .unwrap_or_default();
                CoreError::Operator(format!(
                    "no archived downgrade candidate older than {}{requested} is available for `{}`",
                    installed_version(installed),
                    installed.pkgname
                ))
            })
    }

    fn validate_downgrade_policy(
        &self,
        installed: &elda_db::InstalledPackageDetails,
        candidate: &DowngradeCandidate,
    ) -> Result<(), CoreError> {
        if installed.held {
            return Err(CoreError::Operator(format!(
                "cannot downgrade `{}` while it is held; clear the hold first",
                installed.pkgname
            )));
        }

        if let Some(pinned_version) = &installed.pinned_version
            && pinned_version != &candidate.version().to_string()
        {
            return Err(CoreError::Operator(format!(
                "cannot downgrade `{}` while it is pinned to {}; clear the pin first",
                installed.pkgname, pinned_version
            )));
        }

        Ok(())
    }

    fn validate_downgrade_reverse_dependencies(
        &self,
        package_name: &str,
        candidate: &DowngradeCandidate,
    ) -> Result<(), CoreError> {
        let candidate_version = ConstraintVersion::from(&candidate.version());

        for reverse_dependency in self.database.reverse_dependencies(package_name, false)? {
            let constraint = NamedConstraint::parse_dependency(&reverse_dependency.raw_expr)
                .map_err(|error| {
                    CoreError::Operator(format!(
                        "installed reverse dependency `{}` has invalid constraint `{}`: {error}",
                        reverse_dependency.pkgname, reverse_dependency.raw_expr
                    ))
                })?;
            if !constraint.matches_name(package_name)
                || constraint.matches_version(&candidate_version)
            {
                continue;
            }

            return Err(CoreError::Operator(format!(
                "cannot downgrade `{}` to {} because installed package `{}` requires `{}`",
                package_name,
                candidate.version(),
                reverse_dependency.pkgname,
                reverse_dependency.raw_expr
            )));
        }

        Ok(())
    }
}
