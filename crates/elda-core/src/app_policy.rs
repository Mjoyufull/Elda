use std::collections::{BTreeSet, VecDeque};

use serde_json::json;

use crate::app::AppContext;
use crate::app_parse::installed_version;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_why(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("why requires one package name".to_owned()))?
            .clone();
        let installed = self.ensure_installed(&package_name)?;
        let reverse_dependencies = self.database.reverse_dependencies(&package_name, true)?;

        Ok(CommandReport {
            area: "policy",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("reported install reasoning for `{package_name}`."),
            details: Some(json!({
                "package": package_name,
                "installed": installed,
                "version": installed_version(&installed),
                "reverse_dependencies": reverse_dependencies,
            })),
        })
    }

    pub(crate) fn handle_rdeps(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = self.parse_rdeps_request(&request)?;
        self.ensure_installed(&parsed.package)?;
        let reverse_dependencies = if parsed.recursive {
            self.recursive_reverse_dependencies(&parsed.package, parsed.include_weak)?
        } else {
            self.database
                .reverse_dependencies(&parsed.package, parsed.include_weak)?
        };

        Ok(CommandReport {
            area: "policy",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "reported {} reverse dependency edge(s) for `{}`.",
                reverse_dependencies.len(),
                parsed.package,
            ),
            details: Some(json!({
                "package": parsed.package,
                "recursive": parsed.recursive,
                "include_weak": parsed.include_weak,
                "reverse_dependencies": reverse_dependencies,
            })),
        })
    }

    pub(crate) fn handle_pin(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("pin requires one package name".to_owned()))?
            .clone();
        let installed = self.ensure_installed(&package_name)?;
        let version = installed_version(&installed);
        self.database
            .set_pinned_version(&package_name, Some(version.as_str()))?;

        Ok(CommandReport {
            area: "policy",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("pinned `{package_name}` to {version}."),
            details: Some(json!({
                "package": package_name,
                "pinned_version": version,
            })),
        })
    }

    pub(crate) fn handle_unpin(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("unpin requires one package name".to_owned()))?
            .clone();
        self.ensure_installed(&package_name)?;
        self.database.set_pinned_version(&package_name, None)?;

        Ok(CommandReport {
            area: "policy",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("cleared the pinned version for `{package_name}`."),
            details: Some(json!({ "package": package_name, "pinned_version": null })),
        })
    }

    pub(crate) fn handle_hold(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = self.parse_hold_request(&request)?;
        self.ensure_installed(&parsed.package)?;
        self.database
            .set_hold(&parsed.package, true, parsed.source.as_deref())?;

        Ok(CommandReport {
            area: "policy",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("held `{}` against upgrades.", parsed.package),
            details: Some(json!({
                "package": parsed.package,
                "held": true,
                "hold_source": parsed.source,
            })),
        })
    }

    pub(crate) fn handle_unhold(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("unhold requires one package name".to_owned()))?
            .clone();
        self.ensure_installed(&package_name)?;
        self.database.set_hold(&package_name, false, None)?;

        Ok(CommandReport {
            area: "policy",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("cleared the upgrade hold for `{package_name}`."),
            details: Some(json!({
                "package": package_name,
                "held": false,
                "hold_source": null,
            })),
        })
    }

    pub(crate) fn recursive_reverse_dependencies(
        &self,
        package_name: &str,
        include_weak: bool,
    ) -> Result<Vec<elda_db::ReverseDependencyRecord>, CoreError> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([package_name.to_owned()]);
        let mut results = Vec::new();

        while let Some(current) = queue.pop_front() {
            for dependency in self.database.reverse_dependencies(&current, include_weak)? {
                if seen.insert(dependency.pkgname.clone()) {
                    queue.push_back(dependency.pkgname.clone());
                    results.push(dependency);
                }
            }
        }

        results.sort_by(|left, right| left.pkgname.cmp(&right.pkgname));

        Ok(results)
    }

    pub(crate) fn orphan_candidates(&self) -> Result<Vec<String>, CoreError> {
        let mut orphans = self
            .database
            .list_installed_packages()?
            .into_iter()
            .filter(|package| package.install_reason == "dep")
            .filter_map(|package| {
                match self.database.reverse_dependencies(&package.pkgname, true) {
                    Ok(reverse_dependencies) if reverse_dependencies.is_empty() => {
                        Some(Ok(package.pkgname))
                    }
                    Ok(_) => None,
                    Err(error) => Some(Err(CoreError::from(error))),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        orphans.sort();

        Ok(orphans)
    }

    pub(crate) fn ensure_installed(
        &self,
        package_name: &str,
    ) -> Result<elda_db::InstalledPackageDetails, CoreError> {
        self.database
            .installed_package(package_name)?
            .ok_or_else(|| {
                CoreError::Operator(format!("package `{package_name}` is not installed"))
            })
    }
}
