use std::fs;
use std::path::Path;

use crate::error::RecipeError;

use super::detect::infer_recipe_name;
use super::model::LegacyPkgdep;

pub(super) fn render_imported_build_lua(recipe_name: &str) -> String {
    format!(
        "-- Imported from pkgit `bldit` for `{}`.\n-- Manual translation into deterministic Elda Lua build steps is still required.\nerror(\"imported pkgit bldit requires manual translation before build use\")\n",
        recipe_name
    )
}

pub(super) fn parse_pkgdeps(path: &Path) -> Result<Vec<LegacyPkgdep>, RecipeError> {
    let content = fs::read_to_string(path)?;
    let mut pkgdeps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let source = parts
            .next()
            .ok_or_else(|| RecipeError::Parse("pkgdeps line must include a source".to_owned()))?;
        let tag = parts.next().map(ToOwned::to_owned);

        pkgdeps.push(LegacyPkgdep {
            raw: trimmed.to_owned(),
            source: source.to_owned(),
            tag,
            package_name: infer_recipe_name(source),
        });
    }

    Ok(pkgdeps)
}

pub(super) fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), RecipeError> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }

    Ok(())
}
