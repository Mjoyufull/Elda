use std::io::{self, IsTerminal, Write};
use std::path::Path;

use super::interbuild_review::interbuild_review_lines;
use super::review_metadata::{
    GeneratedRecipeReview, generated_metadata_targets, render_metadata_review_frame,
};
use super::review_recheck::recheck_after_edit;
use crate::app::{AppContext, PlannedInstallAction};
use crate::app_review::preview_recipe_for_review;
use crate::app_review_memory::{
    load_review_stamp, review_is_unchanged, write_review_stamp, write_review_stamp_with_context,
};
use crate::editor::{open_path_in_editor, open_paths_in_diff_pager};
use crate::error::CoreError;
use crate::{CommandRequest, OutputMode};

impl AppContext {
    pub(crate) fn review_generated_metadata_if_needed(
        &self,
        request: &CommandRequest,
        install_plan: &[PlannedInstallAction],
    ) -> Result<(), CoreError> {
        if request.output_mode != OutputMode::Human || request.dry_run {
            return Ok(());
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Ok(());
        }

        let layout = self.database.layout();
        let recipes_dir = layout.recipes_dir.clone();
        let data_dir = layout.data_dir.clone();
        let pending = generated_metadata_targets(install_plan);
        let mut accepted_generated_metadata = false;
        for plan in pending {
            if review_one_generated_recipe(&recipes_dir, &data_dir, &plan)? {
                accepted_generated_metadata = true;
            }
        }
        for action in interbuild_review_targets(install_plan) {
            review_one_interbuild(&recipes_dir, &data_dir, action)?;
        }

        review_proceed_install_after_metadata_if_needed(
            request,
            install_plan,
            accepted_generated_metadata,
        )?;

        Ok(())
    }

    pub(crate) fn review_bulk_snapshot_if_needed(
        &self,
        request: &CommandRequest,
        snapshots: &[elda_recipe::SnapshotImportReport],
    ) -> Result<(), CoreError> {
        if request.output_mode != OutputMode::Human || request.dry_run || snapshots.is_empty() {
            return Ok(());
        }
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return Ok(());
        }

        for snapshot in snapshots {
            self.review_one_bulk_snapshot(snapshot)?;
        }

        Ok(())
    }

    fn review_one_bulk_snapshot(
        &self,
        snapshot: &elda_recipe::SnapshotImportReport,
    ) -> Result<(), CoreError> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        let stdin = io::stdin();

        loop {
            writeln!(stdout, "{}", self.render_bulk_snapshot_review(snapshot))?;
            stdout.flush()?;

            let mut answer = String::new();
            stdin.read_line(&mut answer)?;

            match parse_review_response(&answer) {
                ReviewResponse::Accept => {
                    self.commit_bulk_snapshot(snapshot)?;
                    return Ok(());
                }
                ReviewResponse::Abort => {
                    return Err(CoreError::Operator(
                        "snapshot import aborted by user".to_owned(),
                    ));
                }
                ReviewResponse::Edit => {
                    open_path_in_editor(&snapshot.staging_dir)?;
                }
                ReviewResponse::Invalid => {
                    writeln!(stdout, "Enter `Y`, `n`, or `e`.")?;
                }
            }
        }
    }

    fn render_bulk_snapshot_review(&self, snapshot: &elda_recipe::SnapshotImportReport) -> String {
        let recipes_dir = &self.database.layout().recipes_dir;
        let existing = snapshot
            .generated_recipes
            .iter()
            .filter(|name| recipes_dir.join(name).exists())
            .count();
        let imported = if snapshot.replace {
            snapshot.generated_recipes.len()
        } else {
            snapshot.generated_recipes.len().saturating_sub(existing)
        };
        let skipped = snapshot.generated_recipes.len().saturating_sub(imported);
        let sample = snapshot
            .generated_recipes
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let more = snapshot.generated_recipes.len().saturating_sub(8);
        let commit = snapshot.source_commit.as_deref().unwrap_or("unknown");
        let discovered = snapshot.discovered;
        let excluded = snapshot.excluded;
        let skipped_existing = snapshot.skipped_existing;

        let mut lines = vec![
            "┌─ Bulk Snapshot Review".to_owned(),
            "├─ Source".to_owned(),
            format!("│  url: {}", snapshot.source_url),
            format!("│  type: {}", snapshot.repository_type),
            format!("│  commit: {commit}"),
            format!("│  staging: {}", snapshot.staging_dir.display()),
            "│".to_owned(),
            "├─ Import plan".to_owned(),
            format!("│  discovered recipes: {discovered}"),
            format!("│  excluded by policy: {excluded}"),
            format!("│  skipped existing local: {skipped_existing}"),
            format!("│  to import: {imported}"),
            format!("│  existing locally: {existing}"),
            format!("│  skipped without --replace: {skipped}"),
            format!("│  replace existing: {}", snapshot.replace),
            "│".to_owned(),
            "├─ Semantics".to_owned(),
            "│  snapshot import: one-time local metadata copy".to_owned(),
            "│  dynamic remote: not configured by this operation".to_owned(),
            "│  review path: edit the staging dir before accepting".to_owned(),
            "│".to_owned(),
            "├─ Preview".to_owned(),
            format!("│  {sample}"),
        ];
        if more > 0 {
            lines.push(format!("│  ... and {more} more"));
        }
        lines.push("└─ Accept and import these recipes? [Y/n/e] ".to_owned());
        lines.join("\n")
    }

    fn commit_bulk_snapshot(
        &self,
        snapshot: &elda_recipe::SnapshotImportReport,
    ) -> Result<(), CoreError> {
        let recipes_dir = &self.database.layout().recipes_dir;
        std::fs::create_dir_all(recipes_dir)?;

        for recipe_name in &snapshot.generated_recipes {
            let source = snapshot.staging_dir.join(recipe_name);
            let target = recipes_dir.join(recipe_name);

            if target.exists() {
                if !snapshot.replace {
                    return Err(CoreError::Operator(format!(
                        "metadata for `{recipe_name}` already exists; pass `--replace` to overwrite it"
                    )));
                }
                std::fs::remove_dir_all(&target)?;
            }
            std::fs::rename(source, target)?;
        }

        let _ = std::fs::remove_dir_all(&snapshot.staging_dir);

        Ok(())
    }
}

fn interbuild_review_targets(install_plan: &[PlannedInstallAction]) -> Vec<&PlannedInstallAction> {
    install_plan
        .iter()
        .filter(|action| {
            matches!(
                action.resolved.selected_source_kind.as_str(),
                "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template"
            )
        })
        .collect()
}

fn review_one_interbuild(
    recipes_dir: &Path,
    data_dir: &Path,
    action: &PlannedInstallAction,
) -> Result<(), CoreError> {
    if review_is_unchanged(
        data_dir,
        &action.package_name,
        "interbuild",
        &action.resolved.recipe.path,
    )? {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    loop {
        let review = interbuild_review_lines(action, data_dir).join("\n");
        writeln!(stdout, "{review}")?;
        let title = format!(
            "Interbuild source review: {} ({})",
            action.package_name, action.resolved.selected_source_kind
        );
        let previous_stamp = load_review_stamp(data_dir, &action.package_name, "interbuild")?;
        if let Some(stamp) = &previous_stamp {
            let previous = std::path::Path::new(&stamp.recipe_path);
            if previous.is_file() && previous != action.resolved.recipe.path {
                open_paths_in_diff_pager(previous, &action.resolved.recipe.path, &title)?;
            } else {
                preview_recipe_for_review(&action.resolved.recipe.path, &title)?;
            }
        } else {
            preview_recipe_for_review(&action.resolved.recipe.path, &title)?;
        }
        writeln!(stdout, "Paging complete. Press `q` to return to Elda.")?;
        write!(stdout, "Proceed? [Y/n/e] ")?;
        stdout.flush()?;

        let mut answer = String::new();
        stdin.read_line(&mut answer)?;

        match parse_review_response(&answer) {
            ReviewResponse::Accept => {
                write_review_stamp_with_context(
                    data_dir,
                    &action.package_name,
                    "interbuild",
                    &action.resolved.recipe.path,
                    action.resolved.source_ref.clone(),
                    action.resolved.remote_name.clone(),
                    Some(action.resolved.selected_source_kind.clone()),
                )?;
                return Ok(());
            }
            ReviewResponse::Abort => {
                return Err(CoreError::Operator(format!(
                    "install aborted during interbuild review for `{}`",
                    action.package_name
                )));
            }
            ReviewResponse::Edit => {
                open_path_in_editor(&action.resolved.recipe.path)?;
                if let Some(rendered) = recheck_after_edit(recipes_dir, &action.package_name)? {
                    writeln!(stdout, "{rendered}")?;
                }
            }
            ReviewResponse::Invalid => {
                writeln!(stdout, "Enter `Y`, `n`, or `e`.")?;
            }
        }
    }
}

fn review_one_generated_recipe(
    recipes_dir: &Path,
    data_dir: &Path,
    plan: &GeneratedRecipeReview,
) -> Result<bool, CoreError> {
    let recipe_path = plan.recipe_dir.join("pkg.lua");
    if review_is_unchanged(data_dir, &plan.recipe_name, "generated", &recipe_path)? {
        return Ok(false);
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    loop {
        let header = render_metadata_review_frame(plan);
        writeln!(stdout, "{header}")?;
        write!(stdout, "Accept generated metadata? [Y/n/e] ")?;
        stdout.flush()?;

        let mut answer = String::new();
        stdin.read_line(&mut answer)?;

        match parse_review_response(&answer) {
            ReviewResponse::Accept => {
                write_review_stamp(data_dir, &plan.recipe_name, "generated", &recipe_path)?;
                return Ok(true);
            }
            ReviewResponse::Abort => {
                return Err(CoreError::Operator(format!(
                    "install aborted after generated metadata review for `{}`; the metadata remains at {}",
                    plan.recipe_name,
                    plan.recipe_dir.display()
                )));
            }
            ReviewResponse::Edit => {
                open_path_in_editor(&plan.recipe_dir)?;
                if let Some(rendered) = recheck_after_edit(recipes_dir, &plan.recipe_name)? {
                    writeln!(stdout, "{rendered}")?;
                }
            }
            ReviewResponse::Invalid => {
                writeln!(stdout, "Enter `Y`, `n`, or `e`.")?;
            }
        }
    }
}

fn root_install_command(command_path: &[String]) -> bool {
    matches!(
        command_path.first().map(String::as_str),
        Some("i" | "ig" | "ib")
    )
}

fn review_proceed_install_after_metadata_if_needed(
    request: &CommandRequest,
    install_plan: &[PlannedInstallAction],
    accepted_generated_metadata: bool,
) -> Result<(), CoreError> {
    if !accepted_generated_metadata || !root_install_command(&request.command_path) {
        return Ok(());
    }
    if request.output_mode != OutputMode::Human || request.dry_run {
        return Ok(());
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(());
    }

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    loop {
        write!(stdout, "{}", render_install_proceed_frame(install_plan))?;
        stdout.flush()?;

        let mut answer = String::new();
        stdin.read_line(&mut answer)?;
        match parse_proceed_response(&answer) {
            Some(true) => return Ok(()),
            Some(false) => {
                return Err(CoreError::Operator(
                    "install stopped after metadata review; rerun when you are ready to build and activate."
                        .to_owned(),
                ));
            }
            None => writeln!(stdout, "Enter `Y` or `n`.")?,
        }
    }
}

fn render_install_proceed_frame(install_plan: &[PlannedInstallAction]) -> String {
    let mut lines = Vec::new();
    lines.push("┌─ Install Review".to_owned());
    lines.push("├─ Metadata".to_owned());
    lines.push("│  accepted: generated package metadata".to_owned());
    lines.push("│".to_owned());
    lines.push("├─ Pending activation".to_owned());
    for action in install_plan.iter().take(12) {
        lines.push(format!(
            "│  {} {} [{} / {}]{}",
            action.package_name,
            action.resolved.recipe.package.version,
            action.resolved.selected_lane,
            action.resolved.selected_source_kind,
            replacement_suffix(action),
        ));
    }
    if install_plan.len() > 12 {
        lines.push(format!("│  ... and {} more", install_plan.len() - 12));
    }
    lines.push("│".to_owned());
    lines.push("└─ Proceed with install? [Y/n] ".to_owned());
    lines.join("\n")
}

fn replacement_suffix(action: &PlannedInstallAction) -> String {
    if action.replaced_packages.is_empty() {
        String::new()
    } else {
        format!(", replaces {}", action.replaced_packages.join(", "))
    }
}

fn parse_proceed_response(input: &str) -> Option<bool> {
    let normalized = input.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "y" | "yes" => Some(true),
        "n" | "no" => Some(false),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewResponse {
    Accept,
    Abort,
    Edit,
    Invalid,
}

fn parse_review_response(input: &str) -> ReviewResponse {
    let normalized = input.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" | "y" | "yes" => ReviewResponse::Accept,
        "n" | "no" => ReviewResponse::Abort,
        "e" | "edit" => ReviewResponse::Edit,
        _ => ReviewResponse::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        ReviewResponse, interbuild_review_lines, interbuild_review_targets, parse_proceed_response,
        parse_review_response, render_install_proceed_frame,
    };
    use crate::app::{PlannedInstallAction, ResolvedInstallTarget};
    use crate::flags::ResolvedFlagState;
    use elda_recipe::{PackageDefinition, RecipeDocument, SourceDefinition};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn parse_proceed_response_accepts_yes_no() {
        assert_eq!(parse_proceed_response("\n"), Some(true));
        assert_eq!(parse_proceed_response("y"), Some(true));
        assert_eq!(parse_proceed_response("n"), Some(false));
        assert_eq!(parse_proceed_response("maybe"), None);
    }

    #[test]
    fn install_proceed_frame_names_replacements() {
        let recipe_dir = PathBuf::from("/tmp/generated/beta-tool");
        let mut action = planned_action("beta-tool", &recipe_dir);
        action.replaced_packages.push("alpha-tool".to_owned());

        let rendered = render_install_proceed_frame(&[action]);

        assert!(rendered.contains("Proceed with install? [Y/n]"));
        assert!(rendered.contains("replaces alpha-tool"));
    }

    #[test]
    fn parse_review_response_accepts_documented_answers() {
        assert_eq!(parse_review_response("\n"), ReviewResponse::Accept);
        assert_eq!(parse_review_response("y"), ReviewResponse::Accept);
        assert_eq!(parse_review_response("n"), ReviewResponse::Abort);
        assert_eq!(parse_review_response("e"), ReviewResponse::Edit);
        assert_eq!(parse_review_response("what"), ReviewResponse::Invalid);
    }

    #[test]
    fn interbuild_review_targets_selects_parser_backed_actions() {
        let recipe_dir = PathBuf::from("/tmp/generated/example");
        let mut action = planned_action("example", &recipe_dir);
        action.resolved.selected_source_kind = "gentoo_overlay".to_owned();

        let actions = [action];
        let pending = interbuild_review_targets(&actions);

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].package_name, "example");
    }

    #[test]
    fn interbuild_review_lines_are_operator_dense() {
        let recipe_dir = PathBuf::from("/tmp/generated/example");
        let mut action = planned_action("example", &recipe_dir);
        action.resolved.selected_source_kind = "nix_flake".to_owned();
        action.resolved.persisted_source_kind = "interbuild".to_owned();

        let rendered =
            interbuild_review_lines(&action, Path::new("/tmp/elda-review-test")).join("\n");

        assert!(rendered.contains("Interbuild source review"));
        assert!(rendered.contains("Provenance: [I] parsed"));
        assert!(rendered.contains("static flake output parser"));
        assert!(rendered.contains("Metadata: installable=default"));
        assert!(rendered.contains("activate:"));
        assert!(rendered.contains("Review Memory"));
        assert!(rendered.contains("Proceed? [Y/n/e]"));
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
