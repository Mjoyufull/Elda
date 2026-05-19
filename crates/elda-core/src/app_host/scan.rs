use std::path::Path;

use elda_recipe::{IssueSeverity, load_recipe};
use serde_json::{Value, json};

use crate::app::AppContext;
use crate::app_host::tree::RecipeTree;
use crate::app_recipe_show::publish_readiness;
use crate::error::CoreError;

#[derive(Debug, Clone)]
pub(crate) struct TreePackageScan {
    pub(crate) package: String,
    pub(crate) status: &'static str,
    pub(crate) parse_errors: Vec<String>,
    pub(crate) blockers: Vec<String>,
    pub(crate) warnings: Vec<String>,
}

pub(crate) fn scan_tree_packages(
    _app: &AppContext,
    tree: &RecipeTree,
    package_names: &[String],
) -> Result<Vec<TreePackageScan>, CoreError> {
    let mut results = Vec::new();
    for package_name in package_names {
        let mut parse_errors = Vec::new();
        let mut blockers = Vec::new();
        let mut warnings = Vec::new();

        let scan = match load_recipe(&tree.packages_dir, package_name).map_err(CoreError::from) {
            Ok(recipe) => {
                let validation_summary = crate::app_recipe_show::validation_summary_for(&recipe);
                parse_errors.extend(
                    validation_summary
                        .issues
                        .iter()
                        .filter(|issue| issue.severity == IssueSeverity::Error)
                        .map(|issue| issue.message.clone()),
                );
                warnings.extend(
                    validation_summary
                        .issues
                        .iter()
                        .filter(|issue| issue.severity == IssueSeverity::Warning)
                        .map(|issue| format!("validation: {}", issue.message)),
                );
                publish_readiness(
                    &recipe.package,
                    &validation_summary,
                    &mut blockers,
                    &mut warnings,
                );
                if parse_errors.is_empty() && blockers.is_empty() {
                    "ready"
                } else if parse_errors.is_empty() {
                    "blocked"
                } else {
                    "error"
                }
            }
            Err(error) => {
                parse_errors.push(error.to_string());
                "error"
            }
        };

        results.push(TreePackageScan {
            package: package_name.clone(),
            status: scan,
            parse_errors,
            blockers,
            warnings,
        });
    }
    Ok(results)
}

pub(crate) fn scan_tree_json(tree_root: &Path, results: &[TreePackageScan]) -> Value {
    json!({
        "tree": tree_root,
        "packages": results.iter().map(|entry| json!({
            "package": entry.package,
            "status": entry.status,
            "parse_errors": entry.parse_errors,
            "blockers": entry.blockers,
            "warnings": entry.warnings,
        })).collect::<Vec<_>>(),
        "summary": {
            "total": results.len(),
            "ready": results.iter().filter(|entry| entry.status == "ready").count(),
            "blocked": results.iter().filter(|entry| entry.status == "blocked").count(),
            "error": results.iter().filter(|entry| entry.status == "error").count(),
        }
    })
}
