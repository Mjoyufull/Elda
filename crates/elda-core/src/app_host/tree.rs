use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::app_ci::workspace::{copy_dir_recursive, git_head_commit, remove_path_if_exists};
// workspace is pub(crate) from app_ci
use crate::error::CoreError;

#[derive(Debug, Clone)]
pub(crate) struct RecipeTree {
    pub(crate) root: PathBuf,
    pub(crate) packages_dir: PathBuf,
    pub(crate) packages_subdir: String,
}

pub(crate) fn resolve_recipe_tree(
    tree_root: &Path,
    packages_subdir: &str,
) -> Result<RecipeTree, CoreError> {
    let tree_root = tree_root.canonicalize().map_err(|error| {
        CoreError::Operator(format!(
            "recipe tree `{}` is not accessible: {error}",
            tree_root.display()
        ))
    })?;

    let nested = tree_root.join(packages_subdir);
    let packages_dir = if nested.is_dir() && has_any_pkg_lua(&nested)? {
        nested
    } else if has_any_pkg_lua(&tree_root)? {
        tree_root.clone()
    } else {
        return Err(CoreError::Operator(format!(
            "no packages found under `{}` or `{}/{}`",
            tree_root.display(),
            tree_root.display(),
            packages_subdir
        )));
    };

    Ok(RecipeTree {
        root: tree_root,
        packages_dir,
        packages_subdir: packages_subdir.to_owned(),
    })
}

pub(crate) fn discover_package_names(
    tree: &RecipeTree,
    only: &[String],
) -> Result<Vec<String>, CoreError> {
    let ignored = load_eldaignore(&tree.root)?;
    let mut names = Vec::new();

    for entry in fs::read_dir(&tree.packages_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if ignored.contains(name) {
            continue;
        }
        if !path.join("pkg.lua").is_file() {
            continue;
        }
        if !only.is_empty() && !only.iter().any(|pkg| pkg == name) {
            continue;
        }
        names.push(name.to_owned());
    }

    names.sort();
    names.dedup();
    if names.is_empty() {
        return Err(CoreError::Operator(format!(
            "no package directories with pkg.lua were found under {}",
            tree.packages_dir.display()
        )));
    }
    Ok(names)
}

pub(crate) fn git_tree_commit(tree_root: &Path) -> Result<Option<String>, CoreError> {
    git_head_commit(tree_root)
}

pub(crate) fn git_changed_packages_since(
    tree: &RecipeTree,
    since_ref: &str,
) -> Result<Vec<String>, CoreError> {
    let packages_prefix = if tree.packages_dir == tree.root {
        "."
    } else {
        tree.packages_subdir.as_str()
    };
    let output = Command::new("git")
        .arg("-C")
        .arg(&tree.root)
        .args([
            "diff",
            "--name-only",
            since_ref,
            "HEAD",
            "--",
            packages_prefix,
        ])
        .output()
        .map_err(|error| CoreError::Operator(format!("failed to run git diff: {error}")))?;
    if !output.status.success() {
        return Err(CoreError::Operator(format!(
            "git diff failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    let mut names = BTreeSet::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let relative = line
            .strip_prefix(&format!("{}/", tree.packages_subdir))
            .or_else(|| line.strip_prefix(&format!("{}/", packages_prefix)))
            .unwrap_or(line);
        let package = relative.split('/').next().unwrap_or(relative);
        if !package.is_empty() && package != "." {
            names.insert(package.to_owned());
        }
    }

    Ok(names.into_iter().collect())
}

pub(crate) fn sync_tree_packages_to_recipes(
    tree: &RecipeTree,
    recipes_dir: &Path,
    package_names: &[String],
) -> Result<Vec<String>, CoreError> {
    let mut synced = Vec::new();
    for package_name in package_names {
        let source = tree.packages_dir.join(package_name);
        if !source.join("pkg.lua").is_file() {
            return Err(CoreError::Operator(format!(
                "tree package `{package_name}` is missing pkg.lua under {}",
                source.display()
            )));
        }
        let destination = recipes_dir.join(package_name);
        remove_path_if_exists(&destination)?;
        copy_dir_recursive(&source, &destination)?;
        synced.push(package_name.clone());
    }
    Ok(synced)
}

fn has_any_pkg_lua(directory: &Path) -> Result<bool, CoreError> {
    if !directory.is_dir() {
        return Ok(false);
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.path().join("pkg.lua").is_file() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn load_eldaignore(tree_root: &Path) -> Result<BTreeSet<String>, CoreError> {
    let path = tree_root.join(".eldaignore");
    if !path.is_file() {
        return Ok(BTreeSet::new());
    }
    let content = fs::read_to_string(path)?;
    Ok(content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
        .collect())
}
