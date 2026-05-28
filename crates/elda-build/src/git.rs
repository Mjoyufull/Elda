use std::path::{Path, PathBuf};
use std::process::Command;

use elda_recipe::{RecipeDocument, ScalarValue, SourceDefinition};

use crate::BuildLineHook;
use crate::error::BuildError;
use crate::process::{emit_build_line, run_command};

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
    allowed_protocols: &[String],
    line_hook: Option<BuildLineHook>,
) -> Result<SourceCheckout, BuildError> {
    if recipe.package.source.kind != "git" {
        return Err(BuildError::Unsupported(format!(
            "source.kind `{}` is not implemented by the current build slice",
            recipe.package.source.kind
        )));
    }

    checkout_source(
        &recipe.package.source,
        work_root,
        offline,
        allowed_protocols,
        line_hook,
    )
}

pub fn checkout_source(
    source: &SourceDefinition,
    work_root: &Path,
    offline: bool,
    allowed_protocols: &[String],
    line_hook: Option<BuildLineHook>,
) -> Result<SourceCheckout, BuildError> {
    let url = string_field(source, "url")?;
    ensure_git_protocol_allowed(url, allowed_protocols)?;
    if offline && !git_source_is_local(url) {
        return Err(BuildError::Unsupported(format!(
            "offline mode cannot fetch git source `{}`",
            redact_url_credentials(url)
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
    emit_build_line(
        &line_hook,
        format!("[Git] cloning {}", redact_url_credentials(url)),
    );
    run_command("git", clone, "cloning git source")?;

    if let Some(rev) = string_field_optional(source, "rev") {
        let mut checkout = Command::new("git");
        checkout.current_dir(&checkout_dir).args(["checkout", rev]);
        emit_build_line(&line_hook, format!("[Git] checking out {rev}"));
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

pub fn ensure_git_protocol_allowed(
    location: &str,
    allowed_protocols: &[String],
) -> Result<(), BuildError> {
    let protocol = classify_git_protocol(location);
    if allowed_protocols
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(protocol))
    {
        return Ok(());
    }

    Err(BuildError::Unsupported(format!(
        "git source `{}` uses `{protocol}` transport, which is not allowed by [git].allowed_protocols",
        redact_url_credentials(location)
    )))
}

pub fn redact_url_credentials(location: &str) -> String {
    let mut redacted = location.to_owned();
    if let Some(query_start) = redacted.find(['?', '#']) {
        redacted.truncate(query_start);
        redacted.push_str("?redacted");
    }

    let Some(scheme_end) = redacted.find("://") else {
        return redacted;
    };
    let authority_start = scheme_end + 3;
    let authority_end = redacted[authority_start..]
        .find('/')
        .map_or(redacted.len(), |offset| authority_start + offset);
    let authority = &redacted[authority_start..authority_end];
    let Some(at_index) = authority.rfind('@') else {
        return redacted;
    };

    let replace_start = authority_start;
    let replace_end = authority_start + at_index + 1;
    redacted.replace_range(replace_start..replace_end, "redacted@");
    redacted
}

fn classify_git_protocol(location: &str) -> &'static str {
    if location.starts_with("file://") || Path::new(location).exists() {
        return "file";
    }
    if location.starts_with("https://") {
        return "https";
    }
    if location.starts_with("http://") {
        return "http";
    }
    if location.starts_with("ssh://") || looks_like_scp_git_url(location) {
        return "ssh";
    }
    if location.starts_with("git://") {
        return "git";
    }

    "unknown"
}

fn looks_like_scp_git_url(location: &str) -> bool {
    let Some((user_host, path)) = location.split_once(':') else {
        return false;
    };
    !user_host.contains('/') && user_host.contains('@') && !path.is_empty()
}

fn git_source_is_local(url: &str) -> bool {
    if url.starts_with("file://") {
        return true;
    }

    Path::new(url).exists()
}

fn string_field<'a>(source: &'a SourceDefinition, key: &str) -> Result<&'a str, BuildError> {
    string_field_optional(source, key).ok_or_else(|| {
        BuildError::Invalid(format!("source.kind `{}` is missing `{key}`", source.kind))
    })
}

fn string_field_optional<'a>(source: &'a SourceDefinition, key: &str) -> Option<&'a str> {
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

#[cfg(test)]
mod tests {
    use super::{ensure_git_protocol_allowed, redact_url_credentials};

    fn default_allowed() -> Vec<String> {
        vec!["https".to_owned(), "ssh".to_owned(), "file".to_owned()]
    }

    #[test]
    fn protocol_policy_allows_default_safe_git_transports() {
        let allowed = default_allowed();

        ensure_git_protocol_allowed("https://example.invalid/repo.git", &allowed)
            .expect("https should be allowed");
        ensure_git_protocol_allowed("git@example.invalid:repo.git", &allowed)
            .expect("ssh scp-like syntax should be allowed");
        ensure_git_protocol_allowed("file:///tmp/repo.git", &allowed)
            .expect("file URLs should be allowed");
    }

    #[test]
    fn redacts_url_credentials_and_query_tokens() {
        assert_eq!(
            redact_url_credentials("https://user:secret@example.invalid/repo.git?token=abc"),
            "https://redacted@example.invalid/repo.git?redacted"
        );
        assert_eq!(
            redact_url_credentials("https://example.invalid/repo.git?token=abc"),
            "https://example.invalid/repo.git?redacted"
        );
    }

    #[test]
    fn protocol_policy_rejects_insecure_git_transports_by_default() {
        let allowed = default_allowed();

        assert!(ensure_git_protocol_allowed("http://example.invalid/repo.git", &allowed).is_err());
        assert!(ensure_git_protocol_allowed("git://example.invalid/repo.git", &allowed).is_err());
    }
}
