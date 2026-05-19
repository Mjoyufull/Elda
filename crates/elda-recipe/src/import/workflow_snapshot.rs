use std::fs;
use std::path::Path;

use crate::error::RecipeError;

use super::legacy::copy_dir_recursive;
use super::model::{GitRefKind, GitRefRequest, ImportOptions, SnapshotImportReport};
use super::render::render_pkg_lua;

pub(super) fn import_snapshot(
    recipes_dir: &Path,
    source_url: &str,
    source_dir: &Path,
    kind: super::snapshot::SnapshotType,
    options: &ImportOptions,
) -> Result<SnapshotImportReport, RecipeError> {
    let staging_base = Path::new("/var/lib/elda/staging/metadata-import");
    fs::create_dir_all(staging_base)?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| RecipeError::InvalidInput(format!("system clock before UNIX_EPOCH: {err}")))?
        .as_millis();
    let staging_dir = staging_base.join(format!("import-{}", timestamp));
    fs::create_dir_all(&staging_dir)?;

    let candidates = super::snapshot::scan_snapshot(source_dir, kind)?;
    let discovered = candidates.len();
    let mut excluded = 0_usize;
    let mut skipped_existing = 0_usize;
    let mut generated_recipes = Vec::new();
    let source_commit = extract_commit(source_dir);

    for candidate in candidates {
        if options.exclude.contains(&candidate.name) {
            excluded += 1;
            continue;
        }
        if !options.replace && recipes_dir.join(&candidate.name).exists() {
            skipped_existing += 1;
            continue;
        }

        let recipe_staging_dir = staging_dir.join(&candidate.name);
        fs::create_dir_all(&recipe_staging_dir)?;

        let strategy = match kind {
            super::snapshot::SnapshotType::Void => super::strategy::SourceStrategy::XbpsTemplate {
                package: candidate.rel_path.to_string_lossy().to_string(),
            },
            super::snapshot::SnapshotType::Gentoo => {
                super::strategy::SourceStrategy::GentooEbuild {
                    package: candidate.rel_path.to_string_lossy().to_string(),
                }
            }
            super::snapshot::SnapshotType::Elda => super::strategy::SourceStrategy::EldaNative {
                package: Some(candidate.rel_path.to_string_lossy().to_string()),
            },
        };

        let metadata =
            super::metadata::read_generated_metadata(Some(&candidate.source_path), &strategy);

        // For snapshots, we generate a pkg.lua in the staging dir
        // Elda monorepos: copy existing files
        if kind == super::snapshot::SnapshotType::Elda {
            copy_dir_recursive(&candidate.source_path, &recipe_staging_dir)?;
        } else {
            // Foreign: generate pkg.lua that points back to the snapshot repo via subdir
            let subdir = candidate
                .source_path
                .strip_prefix(source_dir)
                .ok()
                .map(|p| p.to_string_lossy().to_string());

            // We need to inject subdir and rev into the generated pkg.lua
            // Since render_pkg_lua is designed for single-recipe git URLs, we might need a
            // slightly different approach or a way to pass these fields.
            // For now, let's use a GitRefRequest for the rev.
            let git_ref = source_commit.as_ref().map(|rev| GitRefRequest {
                kind: GitRefKind::Rev,
                value: rev.clone(),
            });

            // Handle subdir: we'll use a hack by appending it to extra_fields if we can,
            // or just rely on render_pkg_lua's default and then post-process or fix strategy.
            // Actually, let's just write a custom render for snapshot foreigns if needed.
            let pkg_lua = render_pkg_lua(
                &candidate.name,
                Some(source_url),
                &[], // no legacy pkgdeps needed here
                "package",
                &strategy,
                &metadata,
                git_ref.as_ref(),
            );

            // Inject subdir if present
            // Inject subdir if present (but NOT for foreign strategies that handle their own navigation)
            let pkg_lua = if let Some(subdir) = subdir
                && kind == super::snapshot::SnapshotType::Elda
            {
                pkg_lua.replace(
                    &format!("kind = \"{}\",", strategy.kind()),
                    &format!(
                        "kind = \"{}\",\n    subdir = \"{}\",",
                        strategy.kind(),
                        subdir
                    ),
                )
            } else {
                pkg_lua
            };

            fs::write(recipe_staging_dir.join("pkg.lua"), pkg_lua)?;
        }

        generated_recipes.push(candidate.name);
    }

    Ok(SnapshotImportReport {
        source_url: source_url.to_owned(),
        replace: options.replace,
        source_commit,
        repository_type: kind.display().to_owned(),
        discovered,
        excluded,
        skipped_existing,
        to_import: generated_recipes.len(),
        generated_recipes,
        staging_dir,
    })
}

fn extract_commit(dir: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .ok()?;
    if output.status.success() {
        return Some(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    None
}
