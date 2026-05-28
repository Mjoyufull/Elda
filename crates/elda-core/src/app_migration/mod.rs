mod adapters;
mod reports;

use std::collections::BTreeMap;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest};
use adapters::{ForeignPackage, read_foreign_package, read_foreign_packages};
use elda_db::{InstallRecord, PackageDependencyRecord, PackageFileRecord};
use reports::{adopt_report, migration_from_report, migration_lock_blocked_report};

#[derive(Debug, Clone)]
struct ParsedAdoptRequest {
    source_pm: String,
    package: String,
}

impl AppContext {
    pub(crate) fn handle_adopt(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = parse_adopt_request(&request)?;
        let package = read_foreign_package(
            self.database.layout().root_dir.as_path(),
            &parsed.source_pm,
            &parsed.package,
        )?
        .ok_or_else(|| {
            CoreError::Operator(format!(
                "package `{}` was not found in `{}` database",
                parsed.package, parsed.source_pm
            ))
        })?;

        self.validate_adoptable_package(&package)?;
        if request.dry_run {
            return Ok(adopt_report(request, &package, "planned", false));
        }

        self.record_adopted_package(&package)?;
        Ok(adopt_report(request, &package, "ok", true))
    }

    pub(crate) fn handle_migration_namespace(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        match request.command_path.as_slice() {
            [namespace, command] if namespace == "mg" && command == "from" => {
                self.handle_migration_from(request)
            }
            [namespace, command]
                if namespace == "mg" && (command == "lock" || command == "unlock") =>
            {
                Ok(migration_lock_blocked_report(request))
            }
            _ => Ok(self.handle_stub(request)),
        }
    }

    fn handle_migration_from(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let source_pm = parse_migration_source(&request)?;
        let packages =
            read_foreign_packages(self.database.layout().root_dir.as_path(), &source_pm)?;
        if packages.is_empty() {
            return Err(CoreError::Operator(format!(
                "no installed packages found in `{source_pm}` database"
            )));
        }

        self.validate_migration_batch(&packages)?;
        if !request.dry_run {
            for package in &packages {
                self.record_adopted_package(package)?;
            }
        }

        Ok(migration_from_report(request, &source_pm, &packages))
    }

    fn validate_migration_batch(&self, packages: &[ForeignPackage]) -> Result<(), CoreError> {
        let mut seen_packages = BTreeMap::<&str, usize>::new();
        let mut seen_paths = BTreeMap::<&str, &str>::new();
        for package in packages {
            if let Some(previous) = seen_packages.insert(&package.name, 1) {
                return Err(CoreError::Operator(format!(
                    "foreign database contains duplicate package `{}` ({previous})",
                    package.name
                )));
            }
            self.validate_adoptable_package(package)?;
            for path in &package.files {
                if let Some(owner) = seen_paths.insert(path, &package.name) {
                    return Err(CoreError::Operator(format!(
                        "foreign database has overlapping ownership for `{path}`: `{owner}` and `{}`",
                        package.name
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_adoptable_package(&self, package: &ForeignPackage) -> Result<(), CoreError> {
        if self.database.installed_package(&package.name)?.is_some() {
            return Err(CoreError::Operator(format!(
                "cannot adopt `{}` because Elda already owns that package identity",
                package.name
            )));
        }
        let conflicts = package
            .files
            .iter()
            .filter_map(|path| match self.database.path_owners(path) {
                Ok(owners) if !owners.is_empty() => Some(format!(
                    "{path} owned by {}",
                    owners
                        .into_iter()
                        .map(|owner| owner.pkgname)
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                Ok(_) => None,
                Err(error) => Some(format!("{path} ownership lookup failed: {error}")),
            })
            .collect::<Vec<_>>();
        if !conflicts.is_empty() {
            return Err(CoreError::Operator(format!(
                "cannot adopt `{}` because managed path conflicts exist: {}",
                package.name,
                conflicts.join("; ")
            )));
        }
        Ok(())
    }

    fn record_adopted_package(&self, package: &ForeignPackage) -> Result<(), CoreError> {
        let install = InstallRecord {
            pkgname: package.name.clone(),
            epoch: package.version.epoch,
            pkgver: package.version.pkgver.clone(),
            pkgrel: package.version.pkgrel,
            arch: package.arch.clone(),
            package_kind: "normal".to_owned(),
            variant_id: None,
            install_reason: "explicit".to_owned(),
            source_kind: "adopted".to_owned(),
            source_ref: Some(format!("{}:{}", package.source_pm, package.name)),
            remote_name: package.source_repo.clone(),
            channel: package.source_channel.clone(),
            state_id: None,
            activation_backend: Some("adopted-live-root".to_owned()),
            repo_commit: None,
            payload_sha256: None,
            manifest_hash: None,
            pinned_version: None,
            held: false,
            hold_source: None,
        };
        let files = package
            .files
            .iter()
            .map(|path| PackageFileRecord {
                pkgname: package.name.clone(),
                arch: package.arch.clone(),
                path: path.clone(),
                path_kind: path_kind(path).to_owned(),
                sha256: None,
                size: 0,
                mode: 0,
                link_target: None,
                is_conffile: path.starts_with("/etc/"),
            })
            .collect::<Vec<_>>();
        let dependencies = package
            .dependencies
            .iter()
            .map(|dependency| PackageDependencyRecord {
                pkgname: package.name.clone(),
                dependency_name: dependency_name(dependency),
                dependency_kind: "depends".to_owned(),
                raw_expr: dependency.clone(),
                is_weak: false,
                provider_group: None,
            })
            .collect::<Vec<_>>();
        self.database
            .record_install(&install, &files, &dependencies)?;
        Ok(())
    }
}

fn parse_adopt_request(request: &CommandRequest) -> Result<ParsedAdoptRequest, CoreError> {
    let mut source_pm = None;
    let mut package = None;
    let mut operands = request.operands.iter();

    while let Some(operand) = operands.next() {
        if operand == "--from" {
            source_pm = Some(
                operands
                    .next()
                    .ok_or_else(|| {
                        CoreError::Operator("adopt `--from` requires a package manager".to_owned())
                    })?
                    .clone(),
            );
        } else if let Some(value) = operand.strip_prefix("--from=") {
            source_pm = Some(value.to_owned());
        } else if package.is_none() {
            package = Some(operand.clone());
        } else {
            return Err(CoreError::Operator(format!(
                "unexpected adopt operand `{operand}`"
            )));
        }
    }

    Ok(ParsedAdoptRequest {
        source_pm: normalize_pm_name(&source_pm.ok_or_else(|| {
            CoreError::Operator("adopt requires `--from <package-manager>`".to_owned())
        })?),
        package: package
            .ok_or_else(|| CoreError::Operator("adopt requires one package".to_owned()))?,
    })
}

fn parse_migration_source(request: &CommandRequest) -> Result<String, CoreError> {
    let mut operands = request.operands.iter();
    let source = operands.next().ok_or_else(|| {
        CoreError::Operator("mg from requires one package-manager name".to_owned())
    })?;
    if let Some(extra) = operands.next() {
        return Err(CoreError::Operator(format!(
            "unexpected migration operand `{extra}`"
        )));
    }
    Ok(normalize_pm_name(source))
}

fn normalize_pm_name(name: &str) -> String {
    match name.trim() {
        "dpkg" => "apt".to_owned(),
        other => other.to_owned(),
    }
}

fn path_kind(path: &str) -> &'static str {
    if path.ends_with('/') { "dir" } else { "file" }
}

fn dependency_name(raw: &str) -> String {
    raw.split(['<', '>', '=', ':', ' '])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(raw)
        .to_owned()
}
