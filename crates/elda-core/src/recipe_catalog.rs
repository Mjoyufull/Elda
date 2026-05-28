use std::fs;
use std::path::Path;

use elda_recipe::parse_pkg_lua;
use elda_repo::{RepoError, load_snapshot};
use serde::Serialize;

use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RecipeCatalogEntry {
    pub pkgname: String,
    pub version: Option<String>,
    pub source: String,
    pub description: Option<String>,
    pub licenses: Vec<String>,
    pub upstream: Option<String>,
}

pub(crate) fn validate_recipe_pkgname(pkgname: &str) -> Result<(), CoreError> {
    let trimmed = pkgname.trim();
    if trimmed.is_empty() {
        return Err(CoreError::Operator(
            "package name must not be empty".to_owned(),
        ));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(CoreError::Operator(format!(
            "invalid package name `{pkgname}`: path separators are not allowed"
        )));
    }
    Ok(())
}

pub(crate) fn list_local_recipe_names(recipes_dir: &Path) -> Result<Vec<String>, CoreError> {
    let mut names = Vec::new();
    if !recipes_dir.is_dir() {
        return Ok(names);
    }

    for entry in fs::read_dir(recipes_dir).map_err(CoreError::Io)? {
        let entry = entry.map_err(CoreError::Io)?;
        let path = entry.path();
        if path.is_dir() && path.join("pkg.lua").is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }

    names.sort();
    Ok(names)
}

pub(crate) fn list_local_recipe_entries(
    recipes_dir: &Path,
) -> Result<Vec<RecipeCatalogEntry>, CoreError> {
    let names = list_local_recipe_names(recipes_dir)?;
    let mut entries = Vec::with_capacity(names.len());
    for name in names {
        let path = recipes_dir.join(&name).join("pkg.lua");
        let recipe_content = fs::read_to_string(&path).map_err(CoreError::Io)?;
        let recipe = parse_pkg_lua(&path, &recipe_content)?;
        entries.push(RecipeCatalogEntry {
            pkgname: recipe.package.name.clone(),
            version: Some(format!(
                "{}:{}-{}",
                recipe.package.epoch, recipe.package.version, recipe.package.rel
            )),
            source: "local_recipe".to_owned(),
            description: recipe.package.description.clone(),
            licenses: recipe.package.licenses.clone(),
            upstream: recipe.package.upstream.clone(),
        });
    }
    entries.sort_by(|left, right| left.pkgname.cmp(&right.pkgname));
    Ok(entries)
}

pub(crate) fn list_synced_pkg_entries(
    snapshot_path: &Path,
) -> Result<Vec<RecipeCatalogEntry>, CoreError> {
    let snapshot = match load_snapshot(snapshot_path) {
        Ok(snapshot) => snapshot,
        Err(RepoError::SnapshotMissing) => return Ok(Vec::new()),
        Err(error) => return Err(CoreError::Repo(error)),
    };

    let mut entries = snapshot
        .packages
        .into_iter()
        .map(|record| {
            let version = record.version_string();
            let pkgname = record.pkgname.clone();
            let licenses = record
                .license
                .as_deref()
                .map(|value| vec![value.to_owned()])
                .unwrap_or_default();
            RecipeCatalogEntry {
                pkgname,
                version: Some(version),
                source: format!("synced:{}", record.remote_name),
                description: record.description.or(record.summary),
                licenses,
                upstream: record.homepage,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.pkgname.cmp(&right.pkgname));
    entries.dedup_by(|left, right| left.pkgname == right.pkgname && left.source == right.source);
    Ok(entries)
}

pub(crate) fn remove_local_recipe_directory(
    recipes_dir: &Path,
    pkgname: &str,
) -> Result<std::path::PathBuf, CoreError> {
    validate_recipe_pkgname(pkgname)?;
    let recipe_dir = recipes_dir.join(pkgname);
    if !recipe_dir.join("pkg.lua").is_file() {
        return Err(CoreError::Operator(format!(
            "no local recipe tree at `{}` (missing pkg.lua)",
            recipe_dir.display()
        )));
    }

    fs::remove_dir_all(&recipe_dir).map_err(CoreError::Io)?;
    Ok(recipe_dir)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{list_local_recipe_names, remove_local_recipe_directory, validate_recipe_pkgname};

    #[test]
    fn validate_recipe_pkgname_rejects_path_segments() {
        assert!(validate_recipe_pkgname("a/b").is_err());
        assert!(validate_recipe_pkgname("..").is_err());
        assert!(validate_recipe_pkgname("ok").is_ok());
    }

    #[test]
    fn list_and_remove_round_trip() {
        let temp = TempDir::new().expect("tempdir");
        let recipes = temp.path().join("recipes");
        fs::create_dir_all(recipes.join("demo")).expect("mkdir");
        fs::write(recipes.join("demo/pkg.lua"), "pkg = { name = 'demo' }").expect("write");

        let names = list_local_recipe_names(&recipes).expect("list");
        assert_eq!(names, vec!["demo".to_owned()]);

        let removed = remove_local_recipe_directory(&recipes, "demo").expect("rm");
        assert_eq!(removed, recipes.join("demo"));
        assert!(!recipes.join("demo").exists());
    }
}
