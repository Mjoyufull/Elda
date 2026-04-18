use std::collections::BTreeSet;

use crate::CommandRequest;
use crate::app::ResolvedProfileState;
use crate::error::CoreError;

use super::dedupe_preserve_order;
use super::policy::ProfilePolicyResolution;

#[derive(Debug, Clone)]
pub(crate) struct ParsedProfileSelectionRequest {
    pub(crate) profiles: Vec<String>,
    pub(crate) init: Option<String>,
    pub(crate) native_arch: Option<String>,
    pub(crate) foreign_arches: Vec<String>,
}

pub(crate) fn parse_profile_selection_request(
    request: &CommandRequest,
    command_label: &str,
) -> Result<ParsedProfileSelectionRequest, CoreError> {
    let mut profiles = Vec::new();
    let mut init = None;
    let mut native_arch = None;
    let mut foreign_arches = Vec::new();
    let mut operands = request.operands.iter();

    while let Some(operand) = operands.next() {
        match operand.as_str() {
            "--init" => {
                let value = operands.next().ok_or_else(|| {
                    CoreError::Operator(format!(
                        "{command_label} requires one provider value after `--init`"
                    ))
                })?;
                if value.trim().is_empty() {
                    return Err(CoreError::Operator(format!(
                        "invalid empty init-provider for `{command_label} --init`"
                    )));
                }
                if init.replace(value.clone()).is_some() {
                    return Err(CoreError::Operator(format!(
                        "{command_label} accepts at most one `--init` value"
                    )));
                }
            }
            "--native-arch" => {
                let value = operands.next().ok_or_else(|| {
                    CoreError::Operator(format!(
                        "{command_label} requires one architecture value after `--native-arch`"
                    ))
                })?;
                validate_arch(value, command_label, "--native-arch")?;
                if native_arch.replace(value.clone()).is_some() {
                    return Err(CoreError::Operator(format!(
                        "{command_label} accepts at most one `--native-arch` value"
                    )));
                }
            }
            "--foreign-arch" => {
                let value = operands.next().ok_or_else(|| {
                    CoreError::Operator(format!(
                        "{command_label} requires one architecture value after `--foreign-arch`"
                    ))
                })?;
                validate_arch(value, command_label, "--foreign-arch")?;
                foreign_arches.push(value.clone());
            }
            other if other.starts_with("--") => {
                return Err(CoreError::Operator(format!(
                    "unexpected `{command_label}` flag `{other}`"
                )));
            }
            _ => profiles.push(operand.clone()),
        }
    }

    if profiles.is_empty() {
        return Err(CoreError::Operator(format!(
            "{command_label} requires at least one profile anchor"
        )));
    }

    Ok(ParsedProfileSelectionRequest {
        profiles,
        init,
        native_arch,
        foreign_arches,
    })
}

pub(crate) fn ensure_active_profiles_exist(
    active_profiles: &[String],
    requested_profiles: &[String],
) -> Result<(), CoreError> {
    let active = active_profiles.iter().cloned().collect::<BTreeSet<_>>();
    let missing = requested_profiles
        .iter()
        .filter(|profile| !active.contains(*profile))
        .cloned()
        .collect::<Vec<_>>();

    if missing.is_empty() {
        return Ok(());
    }

    Err(CoreError::Operator(format!(
        "cannot remove inactive profile anchor(s): `{}`",
        missing.join("`, `"),
    )))
}

pub(crate) fn next_foreign_arches(
    current: &ResolvedProfileState,
    parsed: &ParsedProfileSelectionRequest,
    declared_policy: &ProfilePolicyResolution,
) -> Vec<String> {
    if !parsed.foreign_arches.is_empty() {
        return dedupe_preserve_order(parsed.foreign_arches.clone());
    }
    if !declared_policy.foreign_arches.is_empty() {
        return declared_policy.foreign_arches.clone();
    }

    current.foreign_arches.clone()
}

fn validate_arch(value: &str, command_label: &str, flag: &str) -> Result<(), CoreError> {
    const CANONICAL_ARCHES: &[&str] = &["amd64", "i386", "arm64", "armhf", "riscv64", "ppc64le"];

    if value.trim().is_empty() {
        return Err(CoreError::Operator(format!(
            "invalid empty architecture for `{command_label} {flag}`"
        )));
    }
    if !CANONICAL_ARCHES.contains(&value) {
        return Err(CoreError::Operator(format!(
            "{command_label} {flag} requires a canonical Elda architecture label, got `{value}`"
        )));
    }

    Ok(())
}
