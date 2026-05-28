use super::*;

impl AppContext {
    pub(super) fn decision_for_action(
        action: &PlannedUpgradeAction,
    ) -> Result<UpgradeDecision, CoreError> {
        let installed_version_string = action.installed.as_ref().map(installed_version);
        let candidate_version = format!(
            "{}:{}-{}",
            action.resolved.recipe.package.epoch,
            action.resolved.recipe.package.version,
            action.resolved.recipe.package.rel,
        );

        let Some(installed) = &action.installed else {
            return Ok(UpgradeDecision {
                installed_version: None,
                candidate_version,
                selected_lane: action.resolved.selected_lane.clone(),
                needs_change: true,
                change_kind: if action.explicit_target {
                    "install-target"
                } else {
                    "install-dependency"
                },
                blocked_reason: None,
                pinned_version: None,
                hold_source: None,
            });
        };

        if installed.held {
            return Ok(blocked_upgrade_decision(
                action,
                installed_version_string,
                candidate_version,
                installed,
                "held",
            ));
        }

        if let Some(pinned_version) = &installed.pinned_version
            && pinned_version != &candidate_version
        {
            return Ok(blocked_upgrade_decision(
                action,
                installed_version_string,
                candidate_version,
                installed,
                "pinned-version",
            ));
        }

        if let Some(decision) = ad_hoc_git_decision(
            action,
            installed,
            installed_version_string.clone(),
            candidate_version.clone(),
        ) {
            return Ok(decision);
        }

        let candidate = PackageVersion::from_str(&candidate_version)
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        let installed_version_value = PackageVersion::from_str(&installed_version(installed))
            .map_err(|error| CoreError::Operator(error.to_string()))?;
        let installed_variant = installed
            .variant_id
            .clone()
            .unwrap_or_else(|| "default".to_owned());
        let resolved_variant = action.resolved.flag_state.variant_id.clone();
        let variant_changed = installed_variant != resolved_variant;
        let needs_change = candidate > installed_version_value
            || (variant_changed && action.rebuild_variant_drift);

        Ok(UpgradeDecision {
            installed_version: installed_version_string,
            candidate_version,
            selected_lane: action.resolved.selected_lane.clone(),
            needs_change,
            change_kind: change_kind(action.explicit_target, needs_change),
            blocked_reason: None,
            pinned_version: installed.pinned_version.clone(),
            hold_source: installed.hold_source.clone(),
        })
    }

    pub(super) fn upgrade_decision(
        &self,
        action: &PlannedUpgradeAction,
    ) -> Result<UpgradeDecision, CoreError> {
        Self::decision_for_action(action)
    }
}

fn blocked_upgrade_decision(
    action: &PlannedUpgradeAction,
    installed_version: Option<String>,
    candidate_version: String,
    installed: &elda_db::InstalledPackageDetails,
    blocked_reason: &'static str,
) -> UpgradeDecision {
    UpgradeDecision {
        installed_version,
        candidate_version,
        selected_lane: action.resolved.selected_lane.clone(),
        needs_change: false,
        change_kind: "blocked",
        blocked_reason: Some(blocked_reason),
        pinned_version: installed.pinned_version.clone(),
        hold_source: installed.hold_source.clone(),
    }
}

fn ad_hoc_git_decision(
    action: &PlannedUpgradeAction,
    installed: &elda_db::InstalledPackageDetails,
    installed_version: Option<String>,
    candidate_version: String,
) -> Option<UpgradeDecision> {
    if !action.resolved.ad_hoc_git {
        return None;
    }

    let source_ref_changed =
        action.resolved.source_ref.is_some() && installed.source_ref != action.resolved.source_ref;
    let commit_changed = installed.repo_commit.is_some()
        && action.candidate_repo_commit.is_some()
        && installed.repo_commit != action.candidate_repo_commit;
    let needs_change = commit_changed && (action.resolved.ad_hoc_git_moving || source_ref_changed);

    Some(UpgradeDecision {
        installed_version,
        candidate_version,
        selected_lane: action.resolved.selected_lane.clone(),
        needs_change,
        change_kind: change_kind(action.explicit_target, needs_change),
        blocked_reason: None,
        pinned_version: installed.pinned_version.clone(),
        hold_source: installed.hold_source.clone(),
    })
}

fn change_kind(explicit_target: bool, needs_change: bool) -> &'static str {
    if !needs_change {
        return "keep-installed";
    }
    if explicit_target {
        "upgrade-target"
    } else {
        "upgrade-dependency"
    }
}
