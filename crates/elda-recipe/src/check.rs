use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::RecipeError;
use crate::model::{IssueSeverity, RecipeDocument};
use crate::{parse_pkg_lua, validate_recipe};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecipeCheckReport {
    pub checked_root: PathBuf,
    pub requested_recipe: Option<String>,
    pub recipes: Vec<CheckedRecipe>,
    pub issues: Vec<RecipeIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckedRecipe {
    pub name: String,
    pub path: PathBuf,
    pub has_pkg_lua: bool,
    pub has_build_lua: bool,
    pub has_patches_dir: bool,
    pub parsed: bool,
    pub document: Option<RecipeDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecipeIssue {
    pub recipe: Option<String>,
    pub severity: IssueSeverity,
    pub message: String,
}

pub fn check_local_recipes(
    recipes_dir: &Path,
    requested_recipe: Option<&str>,
) -> Result<RecipeCheckReport, RecipeError> {
    let mut recipes = collect_recipe_paths(recipes_dir, requested_recipe)?;
    recipes.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

    let mut checked = Vec::new();
    let mut issues = Vec::new();

    if recipes.is_empty() && requested_recipe.is_none() {
        return Ok(RecipeCheckReport {
            checked_root: recipes_dir.to_path_buf(),
            requested_recipe: None,
            recipes: checked,
            issues,
        });
    }

    for recipe_path in recipes {
        let name = recipe_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "<unknown>".to_owned());
        let recipe = check_recipe_path(&recipe_path, &name, &mut issues)?;
        checked.push(recipe);
    }

    if let Some(recipe) = requested_recipe {
        let found = checked.iter().any(|checked| checked.name == recipe);
        if !found {
            issues.push(RecipeIssue {
                recipe: Some(recipe.to_owned()),
                severity: IssueSeverity::Error,
                message: "requested recipe directory was not found".to_owned(),
            });
        }
    }

    Ok(RecipeCheckReport {
        checked_root: recipes_dir.to_path_buf(),
        requested_recipe: requested_recipe.map(ToOwned::to_owned),
        recipes: checked,
        issues,
    })
}

pub fn load_recipe(recipes_dir: &Path, recipe_name: &str) -> Result<RecipeDocument, RecipeError> {
    let pkg_lua_path = recipes_dir.join(recipe_name).join("pkg.lua");
    let content = fs::read_to_string(&pkg_lua_path)?;
    parse_pkg_lua(&pkg_lua_path, &content)
}

fn check_recipe_path(
    recipe_path: &Path,
    name: &str,
    issues: &mut Vec<RecipeIssue>,
) -> Result<CheckedRecipe, RecipeError> {
    let pkg_lua_path = recipe_path.join("pkg.lua");
    let build_lua_path = recipe_path.join("build.lua");
    let patches_dir = recipe_path.join("patches");
    let mut recipe = CheckedRecipe {
        name: name.to_owned(),
        path: recipe_path.to_path_buf(),
        has_pkg_lua: pkg_lua_path.is_file(),
        has_build_lua: build_lua_path.is_file(),
        has_patches_dir: patches_dir.is_dir(),
        parsed: false,
        document: None,
    };

    if !recipe.has_pkg_lua {
        issues.push(RecipeIssue {
            recipe: Some(name.to_owned()),
            severity: IssueSeverity::Error,
            message: "missing required pkg.lua".to_owned(),
        });
    } else {
        parse_recipe_document(name, &pkg_lua_path, &mut recipe, issues)?;
    }

    push_shape_issue(
        issues,
        name,
        build_lua_path.exists() && !recipe.has_build_lua,
        "build.lua exists but is not a regular file",
    );
    push_shape_issue(
        issues,
        name,
        patches_dir.exists() && !recipe.has_patches_dir,
        "patches exists but is not a directory",
    );

    Ok(recipe)
}

fn parse_recipe_document(
    name: &str,
    pkg_lua_path: &Path,
    recipe: &mut CheckedRecipe,
    issues: &mut Vec<RecipeIssue>,
) -> Result<(), RecipeError> {
    let content = fs::read_to_string(pkg_lua_path)?;
    match parse_pkg_lua(pkg_lua_path, &content) {
        Ok(document) => {
            recipe.parsed = true;
            for issue in validate_recipe(&document) {
                issues.push(RecipeIssue {
                    recipe: Some(name.to_owned()),
                    severity: issue.severity,
                    message: issue.message,
                });
            }
            recipe.document = Some(document);
        }
        Err(error) => {
            issues.push(RecipeIssue {
                recipe: Some(name.to_owned()),
                severity: IssueSeverity::Error,
                message: error.to_string(),
            });
        }
    }

    Ok(())
}

fn push_shape_issue(issues: &mut Vec<RecipeIssue>, name: &str, condition: bool, message: &str) {
    if condition {
        issues.push(RecipeIssue {
            recipe: Some(name.to_owned()),
            severity: IssueSeverity::Error,
            message: message.to_owned(),
        });
    }
}

fn collect_recipe_paths(
    recipes_dir: &Path,
    requested_recipe: Option<&str>,
) -> Result<Vec<PathBuf>, RecipeError> {
    if let Some(recipe) = requested_recipe {
        return Ok(vec![recipes_dir.join(recipe)]);
    }
    if !recipes_dir.exists() {
        return Ok(Vec::new());
    }

    fs::read_dir(recipes_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .filter_map(|entry| match entry {
            Ok(path) if path.is_dir() => Some(Ok(path)),
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(RecipeError::from)
}

#[cfg(test)]
mod tests;
