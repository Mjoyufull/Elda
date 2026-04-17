use std::fs;
use std::path::Path;
use std::process::Command;

pub fn is_git_like_target(input: &str) -> bool {
    input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("git@")
        || input.starts_with("file://")
        || input.ends_with(".git")
}

pub fn infer_recipe_name(source: &str) -> String {
    let trimmed = source.trim_end_matches('/');
    trimmed
        .rsplit('/')
        .next()
        .unwrap_or("new-package")
        .trim_end_matches(".git")
        .to_owned()
}

pub(super) fn detect_default_branch(source_url: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--symref", source_url, "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(remainder) = line.strip_prefix("ref: refs/heads/")
            && let Some((branch, _)) = remainder.split_once('\t')
        {
            return Some(branch.to_owned());
        }
    }

    None
}

pub(super) fn discover_source_url(source_dir: &Path) -> Option<String> {
    let git_config = source_dir.join(".git/config");
    if let Ok(content) = fs::read_to_string(git_config) {
        let mut in_origin = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[remote ") {
                in_origin = trimmed == "[remote \"origin\"]";
                continue;
            }
            if in_origin
                && let Some((key, value)) = trimmed.split_once('=')
                && key.trim() == "url"
            {
                return Some(value.trim().to_owned());
            }
        }
    }

    Some(format!("file://{}", source_dir.display()))
}
