//! Post-edit recipe recheck for the `[Y/n/e]` review gate.
//!
//! When the operator picks `e` in a review prompt, Elda opens the recipe
//! in `$EDITOR`. After the editor closes, this module re-runs the same
//! recipe validation `elda rc check` uses, then renders an operator-dense
//! tree-style block listing every issue with severity glyphs so problems
//! surface as immediate, fixable feedback instead of as opaque later
//! build failures.

use std::path::Path;

use crate::app_render_tree::{Frame, FrameFooter, Glyph, TreeStyle};
use crate::error::CoreError;
use elda_recipe::{IssueSeverity, RecipeIssue, check_local_recipes};

pub(super) fn recheck_after_edit(
    recipes_dir: &Path,
    recipe_name: &str,
) -> Result<Option<String>, CoreError> {
    let report = check_local_recipes(recipes_dir, Some(recipe_name))
        .map_err(|error| CoreError::Operator(format!("recipe recheck failed: {error}")))?;
    let issues: Vec<&RecipeIssue> = report
        .issues
        .iter()
        .filter(|issue| {
            issue
                .recipe
                .as_deref()
                .map(|name| name == recipe_name)
                .unwrap_or(true)
        })
        .collect();

    if issues.is_empty() {
        return Ok(Some(render_recheck_clean(recipe_name)));
    }
    Ok(Some(render_recheck_issues(recipe_name, &issues)))
}

fn render_recheck_clean(recipe_name: &str) -> String {
    let mut frame = Frame::new(format!("Recipe Recheck: {recipe_name}"));
    frame
        .glyph_line(Glyph::Done, "syntax: lua parse ok")
        .glyph_line(Glyph::Done, "schema: required fields present")
        .footer(FrameFooter {
            glyph: Some(Glyph::Done),
            text: "Edit accepted, continuing review".to_owned(),
        });
    frame.render(TreeStyle::detect())
}

fn render_recheck_issues(recipe_name: &str, issues: &[&RecipeIssue]) -> String {
    let mut frame = Frame::new(format!("Recipe Recheck: {recipe_name}"));
    for issue in issues {
        let glyph = match issue.severity {
            IssueSeverity::Error => Glyph::Blocked,
            IssueSeverity::Warning => Glyph::Warn,
        };
        frame.glyph_line(glyph, issue.message.clone());
    }
    frame.footer(FrameFooter {
        glyph: Some(Glyph::Warn),
        text: "Re-open editor (e), abort (n), or accept and continue (Y)".to_owned(),
    });
    frame.render(TreeStyle::detect())
}

#[cfg(test)]
mod tests {
    use super::{render_recheck_clean, render_recheck_issues};
    use elda_recipe::{IssueSeverity, RecipeIssue};

    #[test]
    fn clean_block_announces_acceptance() {
        let rendered = render_recheck_clean("tool");
        assert!(rendered.contains("Recipe Recheck: tool"));
        assert!(rendered.contains("Edit accepted, continuing review"));
    }

    #[test]
    fn issues_block_surfaces_each_issue_with_severity_glyph() {
        let issues = [
            RecipeIssue {
                recipe: Some("tool".to_owned()),
                severity: IssueSeverity::Error,
                message: "missing required field `package.name`".to_owned(),
            },
            RecipeIssue {
                recipe: Some("tool".to_owned()),
                severity: IssueSeverity::Warning,
                message: "license list is empty".to_owned(),
            },
        ];
        let issue_refs: Vec<&RecipeIssue> = issues.iter().collect();
        let rendered = render_recheck_issues("tool", &issue_refs);
        assert!(rendered.contains("missing required field `package.name`"));
        assert!(rendered.contains("license list is empty"));
        assert!(rendered.contains("Re-open editor"));
    }
}
