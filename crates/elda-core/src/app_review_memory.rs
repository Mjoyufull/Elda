use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use std::collections::BTreeSet;

use crate::app::PlannedInstallAction;
use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReviewStamp {
    pub package: String,
    pub review_kind: String,
    pub recipe_hash: String,
    pub recipe_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parser: Option<String>,
}

pub(crate) fn load_review_stamp(
    data_dir: &Path,
    package: &str,
    review_kind: &str,
) -> Result<Option<ReviewStamp>, CoreError> {
    let path = stamp_path(data_dir, package, review_kind);
    if !path.exists() {
        return Ok(None);
    }

    let mut stamp: ReviewStamp = serde_json::from_slice(&fs::read(&path)?)?;
    if stamp.accepted_at.is_none() {
        stamp.accepted_at = stamp_mtime(&path);
    }
    Ok(Some(stamp))
}

pub(crate) fn write_review_stamp(
    data_dir: &Path,
    package: &str,
    review_kind: &str,
    recipe_path: &Path,
) -> Result<ReviewStamp, CoreError> {
    write_review_stamp_with_context(
        data_dir,
        package,
        review_kind,
        recipe_path,
        None,
        None,
        None,
    )
}

pub(crate) fn write_review_stamp_with_context(
    data_dir: &Path,
    package: &str,
    review_kind: &str,
    recipe_path: &Path,
    source_url: Option<String>,
    remote_name: Option<String>,
    parser: Option<String>,
) -> Result<ReviewStamp, CoreError> {
    let stamp = ReviewStamp {
        package: package.to_owned(),
        review_kind: review_kind.to_owned(),
        recipe_hash: hash_file(recipe_path)?,
        recipe_path: recipe_path.display().to_string(),
        accepted_at: None,
        source_url,
        remote_name,
        parser,
    };
    let path = stamp_path(data_dir, package, review_kind);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_vec_pretty(&stamp)?)?;
    append_review_history(data_dir, &stamp, &path)?;

    Ok(stamp)
}

fn append_review_history(
    data_dir: &Path,
    stamp: &ReviewStamp,
    stamp_file: &Path,
) -> Result<(), CoreError> {
    let history_path = data_dir.join("review-stamps").join("history.jsonl");
    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let entry = serde_json::json!({
        "package": stamp.package,
        "review_kind": stamp.review_kind,
        "recipe_hash": stamp.recipe_hash,
        "recipe_path": stamp.recipe_path,
        "source_url": stamp.source_url,
        "remote_name": stamp.remote_name,
        "parser": stamp.parser,
        "accepted_at": stamp_mtime(stamp_file).unwrap_or_else(|| "unknown".to_owned()),
    });
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    Ok(())
}

pub(crate) fn list_review_history(
    data_dir: &Path,
    package: Option<&str>,
) -> Result<Vec<serde_json::Value>, CoreError> {
    let history_path = data_dir.join("review-stamps").join("history.jsonl");
    if !history_path.is_file() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(history_path)?;
    let mut entries = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let entry: serde_json::Value = serde_json::from_str(line)?;
        if package.is_some_and(|name| entry.get("package").and_then(|v| v.as_str()) != Some(name)) {
            continue;
        }
        entries.push(entry);
    }
    entries.reverse();
    Ok(entries.into_iter().take(32).collect())
}

pub(crate) fn review_is_unchanged(
    data_dir: &Path,
    package: &str,
    review_kind: &str,
    recipe_path: &Path,
) -> Result<bool, CoreError> {
    let Some(stamp) = load_review_stamp(data_dir, package, review_kind)? else {
        return Ok(false);
    };

    Ok(stamp.recipe_hash == hash_file(recipe_path)?)
}

pub(crate) fn list_review_stamps(data_dir: &Path) -> Result<Vec<ReviewStamp>, CoreError> {
    let root = data_dir.join("review-stamps");
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut stamps = Vec::new();
    for review_kind in ["generated", "interbuild"] {
        let kind_dir = root.join(review_kind);
        if !kind_dir.is_dir() {
            continue;
        }
        for entry in fs::read_dir(kind_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let mut stamp: ReviewStamp = serde_json::from_slice(&fs::read(&path)?)?;
            if stamp.accepted_at.is_none() {
                stamp.accepted_at = stamp_mtime(&path);
            }
            stamps.push(stamp);
        }
    }

    stamps.sort_by(|left, right| {
        left.package
            .cmp(&right.package)
            .then(left.review_kind.cmp(&right.review_kind))
    });

    Ok(stamps)
}

pub(crate) fn forget_review_stamp(
    data_dir: &Path,
    package: &str,
    review_kind: &str,
) -> Result<bool, CoreError> {
    let path = stamp_path(data_dir, package, review_kind);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

pub(crate) fn install_review_plan_summary(
    data_dir: &Path,
    install_plan: &[PlannedInstallAction],
) -> Result<serde_json::Value, CoreError> {
    let mut entries = Vec::new();
    let mut seen_generated = BTreeSet::new();

    for action in install_plan {
        if let Some(recipe_dir) = &action.resolved.generated_recipe_dir
            && seen_generated.insert(recipe_dir.clone())
        {
            let recipe_name = action
                .resolved
                .generated_recipe_name
                .clone()
                .unwrap_or_else(|| action.package_name.clone());
            entries.push(review_entry(
                data_dir,
                &recipe_name,
                "generated",
                &recipe_dir.join("pkg.lua"),
            )?);
        }

        if matches!(
            action.resolved.selected_source_kind.as_str(),
            "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template"
        ) {
            entries.push(review_entry(
                data_dir,
                &action.package_name,
                "interbuild",
                &action.resolved.recipe.path,
            )?);
        }
    }

    let needs_review = entries
        .iter()
        .filter(|entry| entry["status"] == "needs-review" || entry["status"] == "changed")
        .count();

    Ok(serde_json::json!({
        "entries": entries,
        "needs_review": needs_review,
    }))
}

fn review_entry(
    data_dir: &Path,
    package: &str,
    review_kind: &str,
    recipe_path: &Path,
) -> Result<serde_json::Value, CoreError> {
    let stamp = load_review_stamp(data_dir, package, review_kind)?;
    let unchanged = review_is_unchanged(data_dir, package, review_kind, recipe_path)?;
    let status = if stamp.is_none() {
        "needs-review"
    } else if unchanged {
        "current"
    } else {
        "changed"
    };

    Ok(serde_json::json!({
        "package": package,
        "review_kind": review_kind,
        "status": status,
        "recipe_path": recipe_path.display().to_string(),
        "stamp": stamp,
    }))
}

fn stamp_path(data_dir: &Path, package: &str, review_kind: &str) -> PathBuf {
    data_dir
        .join("review-stamps")
        .join(review_kind)
        .join(format!("{package}.json"))
}

fn hash_file(path: &Path) -> Result<String, CoreError> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);

    Ok(format!("{:x}", hasher.finalize()))
}

fn stamp_mtime(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let duration = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_secs().to_string())
}
