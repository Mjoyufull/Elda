use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;

use crate::app::{AppContext, PlannedInstallAction};
use crate::editor::open_path_in_editor;
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

        let pending = generated_metadata_targets(install_plan);
        for (recipe_name, recipe_dir) in pending {
            review_one_generated_recipe(&recipe_name, &recipe_dir)?;
        }

        Ok(())
    }
}

fn generated_metadata_targets(install_plan: &[PlannedInstallAction]) -> Vec<(String, PathBuf)> {
    let mut seen = BTreeSet::new();
    let mut pending = Vec::new();

    for action in install_plan {
        let Some(recipe_dir) = &action.resolved.generated_recipe_dir else {
            continue;
        };
        if !seen.insert(recipe_dir.clone()) {
            continue;
        }

        pending.push((
            action
                .resolved
                .generated_recipe_name
                .clone()
                .unwrap_or_else(|| action.package_name.clone()),
            recipe_dir.clone(),
        ));
    }

    pending
}

fn review_one_generated_recipe(recipe_name: &str, recipe_dir: &PathBuf) -> Result<(), CoreError> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    let stdin = io::stdin();

    loop {
        writeln!(
            stdout,
            "Generated metadata for `{recipe_name}` is ready.\nPath: {}\n",
            recipe_dir.display()
        )?;
        write!(stdout, "Review before install? [Y/n/e] ")?;
        stdout.flush()?;

        let mut answer = String::new();
        stdin.read_line(&mut answer)?;

        match parse_review_response(&answer) {
            ReviewResponse::Accept => return Ok(()),
            ReviewResponse::Abort => {
                return Err(CoreError::Operator(format!(
                    "install aborted after generated metadata review for `{recipe_name}`; the metadata remains at {}",
                    recipe_dir.display()
                )));
            }
            ReviewResponse::Edit => {
                open_path_in_editor(recipe_dir)?;
            }
            ReviewResponse::Invalid => {
                writeln!(stdout, "Enter `Y`, `n`, or `e`.")?;
            }
        }
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
    use super::{ReviewResponse, generated_metadata_targets, parse_review_response};
    use crate::app::{PlannedInstallAction, ResolvedInstallTarget};
    use crate::flags::ResolvedFlagState;
    use elda_recipe::{PackageDefinition, RecipeDocument, SourceDefinition};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn parse_review_response_accepts_documented_answers() {
        assert_eq!(parse_review_response("\n"), ReviewResponse::Accept);
        assert_eq!(parse_review_response("y"), ReviewResponse::Accept);
        assert_eq!(parse_review_response("n"), ReviewResponse::Abort);
        assert_eq!(parse_review_response("e"), ReviewResponse::Edit);
        assert_eq!(parse_review_response("what"), ReviewResponse::Invalid);
    }

    #[test]
    fn generated_metadata_targets_deduplicates_recipe_dirs() {
        let recipe_dir = PathBuf::from("/tmp/generated/example");
        let actions = vec![
            planned_action("example", &recipe_dir),
            planned_action("example", &recipe_dir),
        ];

        let pending = generated_metadata_targets(&actions);

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0, "example");
        assert_eq!(pending[0].1, recipe_dir);
    }

    fn planned_action(recipe_name: &str, recipe_dir: &PathBuf) -> PlannedInstallAction {
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
                    variant_id: "default".to_owned(),
                    customized: false,
                },
                source_ref: None,
                remote_name: None,
                remote_recipe_source: None,
                binary_source_verification: None,
                ad_hoc_git: true,
                generated_recipe_name: Some(recipe_name.to_owned()),
                generated_recipe_dir: Some(recipe_dir.clone()),
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
