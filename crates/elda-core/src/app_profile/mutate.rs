use serde_json::json;

use crate::app::AppContext;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::empty_to;

impl AppContext {
    pub(crate) fn handle_profile_set_init(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let provider = parse_single_value(&request, "pf set-init", "init-provider")?;
        let mut profile = self.resolve_profile_state()?;
        let previous = profile.init.clone();
        profile.init = provider.clone();

        self.persist_profile_edit(
            request,
            profile,
            format!(
                "planned init-provider change from `{}` to `{}`.",
                empty_to(previous.clone(), "unset".to_owned()),
                provider
            ),
            format!("set the active init-provider family to `{provider}`."),
            json!({
                "kind": "profile-set-init",
                "previous_init": empty_to(previous, "unset".to_owned()),
                "next_init": provider,
            }),
        )
    }

    pub(crate) fn handle_profile_clear_init(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        ensure_no_operands(&request, "pf clear-init")?;
        let mut profile = self.resolve_profile_state()?;
        let previous = profile.init.clone();
        profile.init.clear();

        self.persist_profile_edit(
            request,
            profile,
            format!(
                "planned init-provider change from `{}` to `unset`.",
                empty_to(previous.clone(), "unset".to_owned()),
            ),
            "cleared the active init-provider family.".to_owned(),
            json!({
                "kind": "profile-clear-init",
                "previous_init": empty_to(previous, "unset".to_owned()),
                "next_init": "unset",
            }),
        )
    }

    pub(crate) fn handle_profile_set_arch(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let arch = parse_single_arch(&request, "pf set-arch")?;
        let mut profile = self.resolve_profile_state()?;
        let previous = profile.native_arch.clone();
        profile.native_arch = arch.clone();

        self.persist_profile_edit(
            request,
            profile,
            format!("planned native architecture change from `{previous}` to `{arch}`."),
            format!("set the active native architecture to `{arch}`."),
            json!({
                "kind": "profile-set-arch",
                "previous_native_arch": previous,
                "next_native_arch": arch,
            }),
        )
    }

    pub(crate) fn handle_profile_add_foreign_arch(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let arches = parse_arch_list(&request, "pf add-foreign-arch")?;
        let mut profile = self.resolve_profile_state()?;
        let previous = profile.foreign_arches.clone();
        for arch in arches {
            if !profile.foreign_arches.contains(&arch) {
                profile.foreign_arches.push(arch);
            }
        }
        let next_foreign_arches = profile.foreign_arches.clone();

        self.persist_profile_edit(
            request,
            profile,
            "planned foreign-architecture policy update.".to_owned(),
            "updated the active foreign-architecture set.".to_owned(),
            json!({
                "kind": "profile-add-foreign-arch",
                "previous_foreign_arches": previous,
                "next_foreign_arches": next_foreign_arches,
            }),
        )
    }

    pub(crate) fn handle_profile_remove_foreign_arch(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let arches = parse_arch_list(&request, "pf remove-foreign-arch")?;
        let mut profile = self.resolve_profile_state()?;
        let previous = profile.foreign_arches.clone();
        let mut removed = false;

        for arch in &arches {
            if let Some(index) = profile
                .foreign_arches
                .iter()
                .position(|value| value == arch)
            {
                profile.foreign_arches.remove(index);
                removed = true;
            }
        }

        if !removed {
            return Err(CoreError::Operator(format!(
                "pf remove-foreign-arch did not match any active foreign architecture in `{}`",
                previous.join("`, `"),
            )));
        }
        let next_foreign_arches = profile.foreign_arches.clone();

        self.persist_profile_edit(
            request,
            profile,
            "planned foreign-architecture policy update.".to_owned(),
            "updated the active foreign-architecture set.".to_owned(),
            json!({
                "kind": "profile-remove-foreign-arch",
                "previous_foreign_arches": previous,
                "next_foreign_arches": next_foreign_arches,
            }),
        )
    }

    fn persist_profile_edit(
        &self,
        request: CommandRequest,
        profile: crate::app::ResolvedProfileState,
        dry_run_summary: String,
        success_summary: String,
        details: serde_json::Value,
    ) -> Result<CommandReport, CoreError> {
        let desired = profile.to_desired_profile(self.profile_state_base(&profile)?);
        let provider_reconciliation = if request.dry_run {
            self.plan_profile_backend_state(&desired)?
        } else {
            let reconciliation = self.apply_profile_backend_state(&desired)?;
            self.write_profile_state(&desired)?;
            reconciliation
        };
        let runtime_view = self.profile_runtime_view(&profile)?;
        let detail_key = if request.dry_run { "plan" } else { "change" };
        let flattened = details.clone();
        let mut detail_map = serde_json::Map::new();
        detail_map.insert(detail_key.to_owned(), details);
        if let serde_json::Value::Object(fields) = flattened {
            detail_map.extend(fields);
        }
        detail_map.insert("desired_profile".to_owned(), json!(desired));
        detail_map.insert(
            "provider_reconciliation".to_owned(),
            provider_reconciliation,
        );
        detail_map.insert(
            "provider_families".to_owned(),
            json!(&runtime_view.provider_families),
        );
        detail_map.insert(
            "pending_handler_transitions".to_owned(),
            json!(&runtime_view.pending_handler_transitions),
        );
        detail_map.insert(
            "required_activation_class".to_owned(),
            json!(runtime_view.required_activation_class),
        );
        detail_map.insert("handler_backend".to_owned(), json!(runtime_view.backend));

        if request.dry_run {
            return Ok(CommandReport {
                area: "profile",
                status: "planned",
                exit_status: ExitStatus::Success,
                command_path: request.command_path,
                operands: request.operands,
                output_mode: request.output_mode,
                dry_run: request.dry_run,
                summary: dry_run_summary,
                details: Some(serde_json::Value::Object(detail_map)),
            });
        }

        Ok(CommandReport {
            area: "profile",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: success_summary,
            details: Some(serde_json::Value::Object(detail_map)),
        })
    }
}

fn parse_single_value(
    request: &CommandRequest,
    command_label: &str,
    value_label: &str,
) -> Result<String, CoreError> {
    match request.operands.as_slice() {
        [value] if !value.trim().is_empty() => Ok(value.clone()),
        [] => Err(CoreError::Operator(format!(
            "{command_label} requires one {value_label} value"
        ))),
        [value] => Err(CoreError::Operator(format!(
            "invalid empty {value_label} `{value}`"
        ))),
        [_, extra, ..] => Err(CoreError::Operator(format!(
            "unexpected `{command_label}` operand `{extra}`"
        ))),
    }
}

fn parse_single_arch(request: &CommandRequest, command_label: &str) -> Result<String, CoreError> {
    let arch = parse_single_value(request, command_label, "architecture")?;
    validate_arch(&arch, command_label)?;
    Ok(arch)
}

fn parse_arch_list(
    request: &CommandRequest,
    command_label: &str,
) -> Result<Vec<String>, CoreError> {
    if request.operands.is_empty() {
        return Err(CoreError::Operator(format!(
            "{command_label} requires at least one architecture value"
        )));
    }

    let mut arches = Vec::new();
    for arch in &request.operands {
        validate_arch(arch, command_label)?;
        if !arches.contains(arch) {
            arches.push(arch.clone());
        }
    }

    Ok(arches)
}

fn ensure_no_operands(request: &CommandRequest, command_label: &str) -> Result<(), CoreError> {
    if let Some(extra) = request.operands.first() {
        return Err(CoreError::Operator(format!(
            "{command_label} does not accept operand `{extra}`"
        )));
    }

    Ok(())
}

fn validate_arch(arch: &str, command_label: &str) -> Result<(), CoreError> {
    const CANONICAL_ARCHES: &[&str] = &["amd64", "i386", "arm64", "armhf", "riscv64", "ppc64le"];

    if arch.trim().is_empty() {
        return Err(CoreError::Operator(format!(
            "{command_label} does not accept an empty architecture value"
        )));
    }
    if !CANONICAL_ARCHES.contains(&arch) {
        return Err(CoreError::Operator(format!(
            "{command_label} requires a canonical Elda architecture label, got `{arch}`"
        )));
    }

    Ok(())
}
