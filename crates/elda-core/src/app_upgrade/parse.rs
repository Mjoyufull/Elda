use super::*;

use crate::app::ParsedUpgradeRequest;

impl AppContext {
    pub(super) fn parse_upgrade_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedUpgradeRequest, CoreError> {
        let mut targets = Vec::new();
        let mut refresh_weak_deps = self.config.defaults.refresh_weak_deps;
        let mut rebuild_variant_drift = false;
        let mut git_ref = None;
        let mut operands = request.operands.iter();

        while let Some(operand) = operands.next() {
            match operand.as_str() {
                "--refresh-weak-deps" => refresh_weak_deps = true,
                "--rebuild-variant-drift" => rebuild_variant_drift = true,
                "--to-branch" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-branch` requires one branch name".to_owned())
                    })?;
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Branch,
                        value,
                    )?;
                }
                "--to-tag" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-tag` requires one tag name".to_owned())
                    })?;
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Tag,
                        value,
                    )?;
                }
                "--to-rev" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-rev` requires one revision".to_owned())
                    })?;
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Rev,
                        value,
                    )?;
                }
                _ if operand.starts_with("--to-branch=") => {
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Branch,
                        operand.trim_start_matches("--to-branch="),
                    )?;
                }
                _ if operand.starts_with("--to-tag=") => {
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Tag,
                        operand.trim_start_matches("--to-tag="),
                    )?;
                }
                _ if operand.starts_with("--to-rev=") => {
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Rev,
                        operand.trim_start_matches("--to-rev="),
                    )?;
                }
                _ => targets.push(operand.clone()),
            }
        }

        Ok(ParsedUpgradeRequest {
            targets,
            refresh_weak_deps,
            rebuild_variant_drift,
            git_ref,
        })
    }
}
