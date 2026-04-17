mod apply;
mod set_init;
mod state;

use std::collections::BTreeSet;

use serde_json::json;

use crate::app::ResolvedProfileState;

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

fn profile_details_json(profile: &ResolvedProfileState) -> serde_json::Value {
    json!({
        "active_profiles": profile.active_profiles,
        "native_arch": profile.native_arch,
        "foreign_arches": profile.foreign_arches,
        "provider_families": {
            "init": profile.init,
        },
        "pending_handler_transitions": [],
        "required_activation_class": "none",
    })
}
