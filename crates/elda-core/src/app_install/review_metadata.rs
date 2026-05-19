//! Metadata-review primitives for the `[Y/n/e]` generated-recipe gate.
//!
//! Renders the spec §6.1.0 / §2.2 frame and computes the small derived
//! values (strategy label, missing required fields) shown above the
//! prompt.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::app::PlannedInstallAction;
use crate::app_render_tree::{Frame, FrameFooter, Glyph, TreeStyle};

/// Display-ready summary of one pending generated-recipe review.
#[derive(Debug, Clone)]
pub(super) struct GeneratedRecipeReview {
    pub(super) recipe_name: String,
    pub(super) recipe_dir: PathBuf,
    pub(super) strategy_label: String,
    pub(super) missing_fields: Vec<&'static str>,
}

pub(super) fn generated_metadata_targets(
    install_plan: &[PlannedInstallAction],
) -> Vec<GeneratedRecipeReview> {
    let mut seen = BTreeSet::new();
    let mut pending = Vec::new();

    for action in install_plan {
        let Some(recipe_dir) = &action.resolved.generated_recipe_dir else {
            continue;
        };
        if !seen.insert(recipe_dir.clone()) {
            continue;
        }

        pending.push(GeneratedRecipeReview {
            recipe_name: action
                .resolved
                .generated_recipe_name
                .clone()
                .unwrap_or_else(|| action.package_name.clone()),
            recipe_dir: recipe_dir.clone(),
            strategy_label: format_strategy_label(action),
            missing_fields: missing_review_fields(action),
        });
    }

    pending
}

pub(super) fn format_strategy_label(action: &PlannedInstallAction) -> String {
    let kind = action.resolved.selected_source_kind.as_str();
    let lane = action.resolved.selected_lane.as_str();
    match kind {
        "nix_flake" => "[I] nix_flake (bounded parser)".to_owned(),
        "gentoo_overlay" => "[I] gentoo_overlay (bounded parser)".to_owned(),
        "aur_pkgbuild" => "[I] aur_pkgbuild (bounded parser)".to_owned(),
        "xbps_template" => "[I] xbps_template (bounded parser)".to_owned(),
        "git" => "[V] git (vendor / ad-hoc)".to_owned(),
        _ => format!("[V] {lane}/{kind}"),
    }
}

pub(super) fn missing_review_fields(action: &PlannedInstallAction) -> Vec<&'static str> {
    let mut missing = Vec::new();
    let pkg = &action.resolved.recipe.package;
    if pkg
        .description
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        missing.push("description");
    }
    if pkg.licenses.is_empty() {
        missing.push("licenses");
    }
    if pkg
        .upstream
        .as_deref()
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        missing.push("upstream");
    }

    let kind = action.resolved.selected_source_kind.as_str();
    if matches!(
        kind,
        "github_release" | "release_asset" | "url_archive" | "appimage"
    ) {
        let has_binary = if pkg.source.is_multi_lane() {
            pkg.source
                .lanes
                .get(&action.resolved.selected_lane)
                .is_some_and(|l| l.fields.contains_key("binary"))
        } else {
            pkg.source.fields.contains_key("binary")
        };
        if !has_binary {
            missing.push("binary");
        }
    }

    missing
}

pub(super) fn render_metadata_review_frame(plan: &GeneratedRecipeReview) -> String {
    let mut frame = Frame::new("Metadata Add");
    frame
        .line(format!("Recipe: {}", plan.recipe_name))
        .line(format!("Strategy: {}", plan.strategy_label))
        .line(format!("Output:   {}", plan.recipe_dir.display()));
    if plan.missing_fields.is_empty() {
        frame.glyph_line(Glyph::Done, "Required fields present");
    } else {
        frame.glyph_line(
            Glyph::Warn,
            format!("Missing:  {}", plan.missing_fields.join(", ")),
        );
    }
    frame.footer(FrameFooter {
        glyph: None,
        text: "Review before install".to_owned(),
    });
    frame.render(TreeStyle::detect())
}

#[cfg(test)]
mod tests {
    use super::{
        GeneratedRecipeReview, format_strategy_label, missing_review_fields,
        render_metadata_review_frame,
    };
    use crate::app::{PlannedInstallAction, ResolvedInstallTarget};
    use crate::flags::ResolvedFlagState;
    use elda_recipe::{PackageDefinition, RecipeDocument, SourceDefinition};
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    #[test]
    fn frame_lists_required_fields_when_missing() {
        let plan = GeneratedRecipeReview {
            recipe_name: "tool".to_owned(),
            recipe_dir: PathBuf::from("/etc/elda/recipes/tool"),
            strategy_label: "[I] nix_flake (bounded parser)".to_owned(),
            missing_fields: vec!["description", "licenses"],
        };
        let rendered = render_metadata_review_frame(&plan);
        assert!(rendered.contains("Metadata Add"));
        assert!(rendered.contains("Strategy: [I] nix_flake (bounded parser)"));
        assert!(rendered.contains("Missing:  description, licenses"));
    }

    #[test]
    fn frame_marks_required_fields_present_when_complete() {
        let plan = GeneratedRecipeReview {
            recipe_name: "tool".to_owned(),
            recipe_dir: PathBuf::from("/etc/elda/recipes/tool"),
            strategy_label: "[V] git (vendor / ad-hoc)".to_owned(),
            missing_fields: Vec::new(),
        };
        let rendered = render_metadata_review_frame(&plan);
        assert!(rendered.contains("Required fields present"));
    }

    #[test]
    fn missing_review_fields_reports_blank_description_and_licenses() {
        let recipe_dir = PathBuf::from("/tmp/example");
        let action = planned_action("tool", &recipe_dir);
        let missing = missing_review_fields(&action);
        assert_eq!(missing, vec!["description", "licenses", "upstream"]);
    }

    #[test]
    fn format_strategy_label_uses_provenance_badges() {
        let recipe_dir = PathBuf::from("/tmp/example");
        let mut action = planned_action("tool", &recipe_dir);
        action.resolved.selected_source_kind = "nix_flake".to_owned();
        assert_eq!(
            format_strategy_label(&action),
            "[I] nix_flake (bounded parser)"
        );

        action.resolved.selected_source_kind = "git".to_owned();
        assert_eq!(format_strategy_label(&action), "[V] git (vendor / ad-hoc)");
    }

    fn planned_action(recipe_name: &str, recipe_dir: &Path) -> PlannedInstallAction {
        PlannedInstallAction {
            target: recipe_name.to_owned(),
            package_name: recipe_name.to_owned(),
            resolved: ResolvedInstallTarget {
                recipe: RecipeDocument {
                    path: recipe_dir.join("pkg.lua"),
                    package: PackageDefinition {
                        name: recipe_name.to_owned(),
                        description: None,
                        licenses: Vec::new(),
                        upstream: None,
                        epoch: 0,
                        version: "0.1.0".to_owned(),
                        rel: 1,
                        arch: vec!["amd64".to_owned()],
                        kind: "normal".to_owned(),
                        source: SourceDefinition::single_lane("git".to_owned(), BTreeMap::new()),
                        depends: Vec::new(),
                        makedepends: Vec::new(),
                        checkdepends: Vec::new(),
                        recommends: Vec::new(),
                        suggests: Vec::new(),
                        supplements: Vec::new(),
                        enhances: Vec::new(),
                        provides: Vec::new(),
                        conflicts: Vec::new(),
                        replaces: Vec::new(),
                        conffiles: Vec::new(),
                        sysusers: None,
                        tmpfiles: None,
                        alternatives: None,
                        hooks: None,
                        provider_assets: None,
                        flags_default: None,
                        flags_allowed: None,
                        flags_implies: None,
                        flags_conflicts: None,
                        flags_descriptions: None,
                        flags_required_one_of: None,
                        flags_required_at_most_one: None,
                        flags_required_any_of: None,
                        subpackages: None,
                        profile: None,
                        build: None,
                        has_build_table: false,
                    },
                },
                selected_lane: "source".to_owned(),
                selected_source_kind: "git".to_owned(),
                persisted_source_kind: "git".to_owned(),
                flag_state: ResolvedFlagState {
                    active_profiles: Vec::new(),
                    allowed_flags: Vec::new(),
                    default_flags: BTreeMap::new(),
                    global_flags: BTreeMap::new(),
                    profile_flags: BTreeMap::new(),
                    package_flags: BTreeMap::new(),
                    cli_flags: BTreeMap::new(),
                    effective_flags: BTreeMap::new(),
                    descriptions: BTreeMap::new(),
                    cardinality_groups: Vec::new(),
                    package_flag_layers: Vec::new(),
                    variant_id: "default".to_owned(),
                    customized: false,
                },
                source_ref: None,
                remote_name: None,
                remote_recipe_source: None,
                binary_source_verification: None,
                ad_hoc_git: true,
                ad_hoc_git_moving: true,
                generated_recipe_name: Some(recipe_name.to_owned()),
                generated_recipe_dir: Some(recipe_dir.to_path_buf()),
                source_options: Vec::new(),
                selected_source_option: None,
            },
            replaced_packages: Vec::new(),
            install_reason: "explicit".to_owned(),
            requested_by: None,
            dependency_kind: None,
            raw_expr: None,
            is_weak: false,
            provider_group: None,
            dependencies: Vec::new(),
            already_installed: None,
        }
    }
}
