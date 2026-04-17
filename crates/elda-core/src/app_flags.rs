use std::collections::BTreeMap;

use serde_json::json;

use crate::app::{AppContext, ParsedInstallRequest};
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

impl AppContext {
    pub(crate) fn handle_flag_check(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request.operands.first().cloned();

        let details = if let Some(package_name) = package_name.clone() {
            let resolved =
                self.resolve_install_target(&package_name, &ParsedInstallRequest::default())?;
            json!({
                "package": package_name,
                "selected_lane": resolved.selected_lane,
                "source_kind": resolved.selected_source_kind,
                "flag_state": flag_state_json(&resolved.flag_state),
            })
        } else {
            json!({
                "active_profiles": self.resolve_profile_state()?.active_profiles,
                "global_flags": self.config.flags.global.clone(),
                "profile_flags": self.config.flags.profile.clone(),
                "package_flags": self.config.flags.package.clone(),
            })
        };

        Ok(CommandReport {
            area: "flags",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if package_name.is_some() {
                "reported effective flag state for the requested package.".to_owned()
            } else {
                "reported configured global, profile, and package flag layers.".to_owned()
            },
            details: Some(details),
        })
    }

    pub(crate) fn handle_flag_diff(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let package_name = request.operands.first().cloned();

        let details = if let Some(package_name) = package_name {
            package_flag_diff(self, &package_name)?
        } else {
            installed_flag_drift(self)?
        };

        Ok(CommandReport {
            area: "flags",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "reported flag and variant drift.".to_owned(),
            details: Some(details),
        })
    }
}

fn package_flag_diff(app: &AppContext, package_name: &str) -> Result<serde_json::Value, CoreError> {
    let resolved = app.resolve_install_target(package_name, &ParsedInstallRequest::default())?;
    let installed = app.database.installed_package(package_name)?;

    Ok(json!({
        "package": package_name,
        "installed_variant_id": installed.as_ref().and_then(|package| package.variant_id.clone()),
        "resolved_variant_id": resolved.flag_state.variant_id,
        "variant_changed": installed.as_ref().and_then(|package| package.variant_id.clone())
            != Some(resolved.flag_state.variant_id.clone()),
        "selected_lane": resolved.selected_lane,
        "flag_state": flag_state_json(&resolved.flag_state),
        "changes": flag_changes(
            &resolved.flag_state.default_flags,
            &resolved.flag_state.effective_flags,
        ),
    }))
}

fn installed_flag_drift(app: &AppContext) -> Result<serde_json::Value, CoreError> {
    let installed_packages = app.database.list_installed_packages()?;
    let mut drift = Vec::new();
    let mut unresolved = Vec::new();

    for package in installed_packages {
        let resolved =
            match app.resolve_install_target(&package.pkgname, &ParsedInstallRequest::default()) {
                Ok(resolved) => resolved,
                Err(error) => {
                    unresolved.push(json!({
                        "package": package.pkgname,
                        "reason": error.to_string(),
                    }));
                    continue;
                }
            };
        let installed_variant = package.variant_id.unwrap_or_else(|| "default".to_owned());
        if installed_variant != resolved.flag_state.variant_id {
            drift.push(json!({
                "package": package.pkgname,
                "installed_variant_id": installed_variant,
                "resolved_variant_id": resolved.flag_state.variant_id,
                "selected_lane": resolved.selected_lane,
                "changes": flag_changes(
                    &resolved.flag_state.default_flags,
                    &resolved.flag_state.effective_flags,
                ),
            }));
        }
    }

    Ok(json!({
        "drift": drift,
        "unresolved": unresolved,
    }))
}

fn flag_state_json(state: &crate::flags::ResolvedFlagState) -> serde_json::Value {
    json!({
        "active_profiles": state.active_profiles,
        "allowed_flags": state.allowed_flags,
        "default_flags": state.default_flags,
        "global_flags": state.global_flags,
        "profile_flags": state.profile_flags,
        "package_flags": state.package_flags,
        "cli_flags": state.cli_flags,
        "effective_flags": state.effective_flags,
        "variant_id": state.variant_id,
        "customized": state.customized,
    })
}

fn flag_changes(
    defaults: &BTreeMap<String, bool>,
    effective: &BTreeMap<String, bool>,
) -> Vec<serde_json::Value> {
    effective
        .iter()
        .filter_map(|(flag, enabled)| {
            let default_enabled = defaults.get(flag).copied().unwrap_or(false);
            if default_enabled == *enabled {
                None
            } else {
                Some(json!({
                    "flag": flag,
                    "default": default_enabled,
                    "effective": enabled,
                }))
            }
        })
        .collect()
}
