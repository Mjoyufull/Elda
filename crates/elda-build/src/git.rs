use std::path::{Path, PathBuf};
use std::process::Command;

use elda_recipe::{RecipeDocument, ScalarValue};

use crate::error::BuildError;
use crate::process::run_command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheckout {
    pub source_dir: PathBuf,
    pub repo_commit: Option<String>,
    pub repo_commit_unix: Option<u64>,
}

pub fn checkout_git_source(
    recipe: &RecipeDocument,
    work_root: &Path,
    offline: bool,
) -> Result<SourceCheckout, BuildError> {
    let source = &recipe.package.source;
    if source.kind != "git" {
        return Err(BuildError::Unsupported(format!(
            "source.kind `{}` is not implemented by the current build slice",
            source.kind
        )));
    }

    let url = string_field(source, "url")?;
    if offline && !git_source_is_local(url) {
        return Err(BuildError::Unsupported(format!(
            "offline mode cannot fetch git source `{url}`"
        )));
    }
    let checkout_dir = work_root.join("src");
    let mut clone = Command::new("git");
    clone.arg("clone");

    if let Some(branch) = string_field_optional(source, "branch") {
        clone.args(["--depth", "1", "--branch", branch]);
    } else if let Some(tag) = string_field_optional(source, "tag") {
        clone.args(["--depth", "1", "--branch", tag]);
    }

    clone.arg(url).arg(&checkout_dir);
    run_command("git", clone, "cloning git source")?;

    if let Some(rev) = string_field_optional(source, "rev") {
        let mut checkout = Command::new("git");
        checkout.current_dir(&checkout_dir).args(["checkout", rev]);
        run_command("git", checkout, "checking out requested git revision")?;
    }

    let repo_commit = read_git_output(&checkout_dir, &["rev-parse", "HEAD"])?;
    let repo_commit_unix = read_git_output(&checkout_dir, &["show", "-s", "--format=%ct", "HEAD"])?
        .and_then(|value| value.parse::<u64>().ok());
    let source_dir = match string_field_optional(source, "subdir") {
        Some(subdir) => checkout_dir.join(subdir),
        None => checkout_dir,
    };

    Ok(SourceCheckout {
        source_dir,
        repo_commit,
        repo_commit_unix,
    })
}

fn git_source_is_local(url: &str) -> bool {
    if url.starts_with("file://") {
        return true;
    }

    Path::new(url).exists()
}

fn string_field<'a>(
    source: &'a elda_recipe::SourceDefinition,
    key: &str,
) -> Result<&'a str, BuildError> {
    string_field_optional(source, key).ok_or_else(|| {
        BuildError::Invalid(format!("source.kind `{}` is missing `{key}`", source.kind))
    })
}

fn string_field_optional<'a>(
    source: &'a elda_recipe::SourceDefinition,
    key: &str,
) -> Option<&'a str> {
    match source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn read_git_output(repo_dir: &Path, args: &[&str]) -> Result<Option<String>, BuildError> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(args)
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}
