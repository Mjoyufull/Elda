mod mutate;
mod policy;
mod selection;
mod selection_plan;
mod selection_request;
mod state;
mod system_changes;

use std::collections::BTreeSet;

use serde_json::json;

use crate::app::ResolvedProfileState;
use policy::{ProfilePolicyResolution, profile_policy_json};
pub(crate) use system_changes::{PendingSystemChange, ProfileRuntimeView, ProviderFamilies};

fn empty_to(value: String, fallback: String) -> String {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn dedupe_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }

    deduped
}

fn profile_details_json(
    profile: &ResolvedProfileState,
    declared_policy: &ProfilePolicyResolution,
    runtime_view: &ProfileRuntimeView,
) -> serde_json::Value {
    json!({
        "active_profiles": profile.active_profiles,
        "native_arch": profile.native_arch,
        "foreign_arches": profile.foreign_arches,
        "provider_families": &runtime_view.provider_families,
        "declared_profile_policy": profile_policy_json(declared_policy),
        "pending_handler_transitions": &runtime_view.pending_handler_transitions,
        "required_activation_class": runtime_view.required_activation_class,
        "handler_backend": runtime_view.backend,
    })
}
