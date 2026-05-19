use std::path::Path;

use elda_recipe::PackageDefinition;
use serde_json::{Value, json};

use crate::app::ResolvedInstallTarget;

pub(crate) fn metadata_add_json(
    target: &str,
    resolved: &ResolvedInstallTarget,
    recipe_dir: &Path,
) -> Value {
    let package = &resolved.recipe.package;
    let fields = metadata_field_confidence(package, resolved);
    let missing = fields
        .iter()
        .filter(|field| field["confidence"] == "missing")
        .map(|field| field["field"].clone())
        .collect::<Vec<_>>();

    json!({
        "target": target,
        "recipe_name": package.name,
        "recipe_dir": recipe_dir,
        "pkg_lua": resolved.recipe.path,
        "selected_lane": resolved.selected_lane,
        "selected_source_kind": resolved.selected_source_kind,
        "persisted_source_kind": resolved.persisted_source_kind,
        "source_ref": resolved.source_ref,
        "generated": resolved.generated_recipe_dir.is_some(),
        "source_options": resolved.source_options,
        "selected_source_option": resolved.selected_source_option,
        "fields": fields,
        "missing_fields": missing,
        "publish_ready": missing.is_empty(),
    })
}

fn metadata_field_confidence(
    package: &PackageDefinition,
    resolved: &ResolvedInstallTarget,
) -> Vec<Value> {
    vec![
        field_report("name", !package.name.trim().is_empty(), "authoritative"),
        field_report(
            "version",
            !package.version.trim().is_empty(),
            "authoritative",
        ),
        field_report(
            "description",
            package.description.as_deref().is_some_and(has_text),
            generated_or_authoritative(resolved),
        ),
        field_report(
            "licenses",
            !package.licenses.is_empty(),
            generated_or_authoritative(resolved),
        ),
        field_report(
            "upstream",
            package.upstream.as_deref().is_some_and(has_text),
            generated_or_authoritative(resolved),
        ),
        field_report("source", true, source_confidence(resolved)),
        field_report(
            "dependencies",
            has_dependency_metadata(package),
            relationship_confidence(package),
        ),
        field_report(
            "relationships",
            has_relationship_metadata(package),
            relationship_confidence(package),
        ),
        field_report("variants", has_variant_metadata(package), "derived"),
    ]
}

fn field_report(field: &str, present: bool, confidence: &str) -> Value {
    json!({
        "field": field,
        "confidence": if present { confidence } else { "missing" },
    })
}

fn generated_or_authoritative(resolved: &ResolvedInstallTarget) -> &'static str {
    if resolved.generated_recipe_dir.is_some() {
        "missing"
    } else {
        "authoritative"
    }
}

fn source_confidence(resolved: &ResolvedInstallTarget) -> &'static str {
    match resolved.selected_source_kind.as_str() {
        "nix_flake" | "gentoo_overlay" => "derived",
        _ if resolved.generated_recipe_dir.is_some() => "derived",
        _ => "authoritative",
    }
}

fn relationship_confidence(package: &PackageDefinition) -> &'static str {
    if has_dependency_metadata(package) || has_relationship_metadata(package) {
        "authoritative"
    } else {
        "missing"
    }
}

fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_dependency_metadata(package: &PackageDefinition) -> bool {
    !package.depends.is_empty()
        || !package.makedepends.is_empty()
        || !package.checkdepends.is_empty()
        || !package.recommends.is_empty()
        || !package.suggests.is_empty()
        || !package.supplements.is_empty()
        || !package.enhances.is_empty()
}

fn has_relationship_metadata(package: &PackageDefinition) -> bool {
    !package.provides.is_empty() || !package.conflicts.is_empty() || !package.replaces.is_empty()
}

fn has_variant_metadata(package: &PackageDefinition) -> bool {
    package.flags_default.is_some()
        || package.flags_allowed.is_some()
        || package.flags_implies.is_some()
        || package.flags_conflicts.is_some()
        || package.subpackages.is_some()
        || package.profile.is_some()
}
