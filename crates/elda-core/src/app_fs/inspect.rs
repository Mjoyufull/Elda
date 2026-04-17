use super::*;

impl AppContext {
    pub(crate) fn handle_verify(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let verify_report = verify_packages(&self.database, &request.operands)?;
        let failed = !verify_report.issues.is_empty();

        Ok(CommandReport {
            area: "verify",
            status: if failed { "verify-failed" } else { "ok" },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!(
                "verified {} package(s) across {} managed path(s).",
                verify_report.packages.len(),
                verify_report.checked_paths,
            ),
            details: Some(json!({ "verify_report": verify_report })),
        })
    }

    pub(crate) fn handle_reverify(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.handle_verify(request)
    }

    pub(crate) fn handle_diff(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let parsed = self.parse_diff_request(&request)?;
        self.ensure_installed(&parsed.package)?;

        if parsed.candidate {
            return self.handle_candidate_diff(request, parsed.package);
        }

        let verify_report = verify_packages(&self.database, std::slice::from_ref(&parsed.package))?;
        let changes = verify_report
            .issues
            .iter()
            .map(|issue| {
                json!({
                    "path": issue.path,
                    "change": "live-drift",
                    "detail": issue.detail,
                    "kind": issue.kind,
                })
            })
            .collect::<Vec<_>>();

        Ok(CommandReport {
            area: "diff",
            status: if changes.is_empty() {
                "same"
            } else {
                "different"
            },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("compared live managed files for `{}`.", parsed.package),
            details: Some(json!({
                "package": parsed.package,
                "changes": changes,
            })),
        })
    }

    pub(crate) fn handle_files(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("files requires one package name".to_owned()))?
            .clone();
        let files = self.database.package_files(&package_name)?;

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("listed {} owned path(s) for `{package_name}`.", files.len()),
            details: Some(json!({ "package": package_name, "files": files })),
        })
    }

    pub(crate) fn handle_file_owner(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let path = request
            .operands
            .first()
            .ok_or_else(|| CoreError::Operator("files owner requires one path".to_owned()))?
            .clone();
        let owners = self.database.path_owners(&path)?;

        Ok(CommandReport {
            area: "state",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("reported {} owner(s) for `{path}`.", owners.len()),
            details: Some(json!({ "path": path, "owners": owners })),
        })
    }

    fn handle_candidate_diff(
        &self,
        request: CommandRequest,
        package_name: String,
    ) -> Result<CommandReport, CoreError> {
        let request_shape = self.dependency_install_request(&crate::app::ParsedInstallRequest {
            targets: Vec::new(),
            hard_lane: None,
            preferred_lane: None,
            cli_flag_overrides: Default::default(),
        });
        let resolved = self.resolve_install_target(&package_name, &request_shape)?;
        let built = self.build_resolved_target(&resolved, request.offline)?;
        let installed = self.ensure_installed(&package_name)?;
        let current = self.database.package_files(&package_name)?;
        let current_by_path = current
            .into_iter()
            .map(|entry| (entry.path.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        let candidate_by_path = built
            .package
            .manifest
            .entries
            .iter()
            .map(|entry| (entry.path.clone(), entry))
            .collect::<BTreeMap<_, _>>();
        let paths = current_by_path
            .keys()
            .chain(candidate_by_path.keys())
            .cloned()
            .collect::<BTreeSet<_>>();

        let mut changes = Vec::new();
        for path in paths {
            match (current_by_path.get(&path), candidate_by_path.get(&path)) {
                (None, Some(_)) => changes.push(json!({ "path": path, "change": "add" })),
                (Some(_), None) => changes.push(json!({ "path": path, "change": "remove" })),
                (Some(current), Some(candidate))
                    if current.sha256 != candidate.sha256
                        || current.mode != candidate.mode
                        || current.link_target != candidate.link_target
                        || current.path_kind != manifest_kind(candidate.kind) =>
                {
                    changes.push(json!({ "path": path, "change": "modify" }));
                }
                _ => {}
            }
        }

        Ok(CommandReport {
            area: "diff",
            status: if changes.is_empty() {
                "same"
            } else {
                "different"
            },
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: format!("compared `{package_name}` against the next candidate manifest."),
            details: Some(json!({
                "package": package_name,
                "installed_version": installed_version(&installed),
                "candidate": {
                    "version": format!(
                        "{}:{}-{}",
                        built.package.epoch,
                        built.package.pkgver,
                        built.package.pkgrel,
                    ),
                    "selected_lane": resolved.selected_lane,
                },
                "changes": changes,
            })),
        })
    }
}
