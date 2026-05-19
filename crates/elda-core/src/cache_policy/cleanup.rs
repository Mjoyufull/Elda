use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rustix::fs::statvfs;
use serde::Deserialize;

use crate::error::CoreError;
use elda_build::{cache_metadata_path, load_cache_metadata};
use elda_db::{Database, StateLayout};

use super::{
    CacheCleanupReport, CachePolicy, CachePolicyReport, DeletedCacheEntry, PrunableEntry,
    UsageTotals, build_policy_report, effective_trigger_bytes, usage_bytes,
};

#[derive(Debug, Deserialize)]
struct ArchivedStateRefs {
    packages: Vec<ArchivedStatePackageRef>,
}

#[derive(Debug, Deserialize)]
struct ArchivedStatePackageRef {
    pkgname: String,
    pkgver: String,
    pkgrel: u64,
    arch: String,
}

pub(super) fn build_cache_policy_report(
    database: &Database,
    policy: CachePolicy,
) -> Result<CachePolicyReport, CoreError> {
    Ok(build_policy_report(
        collect_usage_totals(database.layout(), policy)?,
        policy,
    ))
}

pub(super) fn reconcile_cache_policy(
    database: &Database,
    policy: CachePolicy,
) -> Result<CacheCleanupReport, CoreError> {
    let usage = collect_usage_totals(database.layout(), policy)?;
    let effective_trigger_bytes = effective_trigger_bytes(&usage, policy);
    let usage_before_bytes = usage_bytes(&usage);

    if usage_before_bytes <= effective_trigger_bytes {
        return Ok(CacheCleanupReport {
            usage_before_bytes,
            usage_after_bytes: usage_before_bytes,
            effective_trigger_bytes,
            deleted_entries: Vec::new(),
        });
    }

    let retained_payloads = retained_package_basenames(database)?;
    let mut candidates = collect_prunable_entries(database.layout(), &retained_payloads, policy)?;
    candidates.sort_by_key(|candidate| candidate.last_access_unix);

    let mut usage_after_bytes = usage_before_bytes;
    let mut deleted_entries = Vec::new();
    for candidate in candidates {
        if usage_after_bytes <= effective_trigger_bytes {
            break;
        }

        let freed = delete_cache_entry(&candidate)?;
        if freed == 0 {
            continue;
        }

        usage_after_bytes = usage_after_bytes.saturating_sub(freed);
        deleted_entries.push(DeletedCacheEntry {
            path: candidate.display_path.display().to_string(),
            entry_kind: candidate.entry_kind.to_owned(),
            bytes_freed: freed,
        });
    }

    Ok(CacheCleanupReport {
        usage_before_bytes,
        usage_after_bytes,
        effective_trigger_bytes,
        deleted_entries,
    })
}

fn collect_usage_totals(
    layout: &StateLayout,
    policy: CachePolicy,
) -> Result<UsageTotals, CoreError> {
    Ok(UsageTotals {
        package_usage_bytes: directory_usage_bytes(&layout.cache_pkg_dir)?,
        source_usage_bytes: directory_usage_bytes(&layout.cache_src_dir)?,
        package_entry_count: directory_entry_count(&layout.cache_pkg_dir)?,
        source_entry_count: directory_entry_count(&layout.cache_src_dir)?,
        filesystem_trigger_bytes: filesystem_trigger_bytes(&layout.cache_pkg_dir, policy)?,
    })
}

fn retained_package_basenames(database: &Database) -> Result<BTreeSet<String>, CoreError> {
    let mut retained = BTreeSet::new();

    for package in database.list_installed_packages()? {
        let Some(details) = database.installed_package(&package.pkgname)? else {
            continue;
        };
        if let Some(base_name) = package_cache_base_name(
            &details.pkgname,
            &details.pkgver,
            details.pkgrel,
            details.arch.as_deref(),
        ) {
            retained.insert(base_name);
        }
    }

    for archive_path in state_archive_paths(&database.layout().states_dir)? {
        let document = serde_json::from_slice::<ArchivedStateRefs>(&fs::read(&archive_path)?)?;
        for package in document.packages {
            retained.insert(format!(
                "{}-{}-{}-{}",
                package.pkgname, package.pkgver, package.pkgrel, package.arch
            ));
        }
    }

    Ok(retained)
}

fn collect_prunable_entries(
    layout: &StateLayout,
    retained_payloads: &BTreeSet<String>,
    policy: CachePolicy,
) -> Result<Vec<PrunableEntry>, CoreError> {
    let now = current_unix_timestamp()?;
    let mut entries = collect_source_candidates(&layout.cache_src_dir, now, policy)?;
    entries.extend(collect_package_candidates(
        &layout.cache_pkg_dir,
        retained_payloads,
        now,
        policy,
    )?);
    Ok(entries)
}

fn collect_source_candidates(
    cache_src_dir: &Path,
    now: u64,
    policy: CachePolicy,
) -> Result<Vec<PrunableEntry>, CoreError> {
    let mut entries = Vec::new();

    for path in cache_file_paths(cache_src_dir)? {
        let last_access_unix = cache_access_unix(&path)?;
        if now.saturating_sub(last_access_unix) < policy.source_retention_secs {
            continue;
        }

        entries.push(PrunableEntry {
            display_path: path.clone(),
            entry_kind: "source-artifact",
            bytes: file_size(&path)?,
            last_access_unix,
            paths: vec![path.clone(), cache_metadata_path(&path)],
        });
    }

    Ok(entries)
}

fn collect_package_candidates(
    cache_pkg_dir: &Path,
    retained_payloads: &BTreeSet<String>,
    now: u64,
    policy: CachePolicy,
) -> Result<Vec<PrunableEntry>, CoreError> {
    let mut entries = Vec::new();

    for path in cache_file_paths(cache_pkg_dir)? {
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".pkg.tar.zst"))
        {
            continue;
        }

        let Some(base_name) = payload_base_name(&path) else {
            continue;
        };
        if retained_payloads.contains(&base_name) {
            continue;
        }

        let last_access_unix = cache_access_unix(&path)?;
        if now.saturating_sub(last_access_unix) < policy.payload_retention_secs {
            continue;
        }

        let manifest_path = cache_pkg_dir.join(format!("{base_name}.manifest"));
        entries.push(PrunableEntry {
            display_path: path.clone(),
            entry_kind: "package-payload",
            bytes: file_size(&path)? + file_size_if_present(&manifest_path)?,
            last_access_unix,
            paths: vec![
                path.clone(),
                cache_metadata_path(&path),
                manifest_path.clone(),
                cache_metadata_path(&manifest_path),
            ],
        });
    }

    Ok(entries)
}

fn delete_cache_entry(entry: &PrunableEntry) -> Result<u64, CoreError> {
    for path in &entry.paths {
        if path.exists() && path.is_file() {
            fs::remove_file(path)?;
        }
    }

    Ok(entry.bytes)
}

fn filesystem_trigger_bytes(path: &Path, policy: CachePolicy) -> Result<u64, CoreError> {
    let stats = statvfs(path).map_err(std::io::Error::from)?;
    let total_bytes = stats.f_blocks.saturating_mul(stats.f_frsize.max(1));
    Ok(total_bytes.saturating_mul(policy.filesystem_trigger_percent) / 100)
}

fn directory_usage_bytes(directory: &Path) -> Result<u64, CoreError> {
    Ok(cache_file_paths(directory)?
        .into_iter()
        .map(|path| file_size(&path))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .sum())
}

fn directory_entry_count(directory: &Path) -> Result<usize, CoreError> {
    Ok(cache_file_paths(directory)?.len())
}

fn cache_file_paths(directory: &Path) -> Result<Vec<PathBuf>, CoreError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_file()
            && !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".eldameta.json"))
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn cache_access_unix(path: &Path) -> Result<u64, CoreError> {
    if let Some(metadata) = load_cache_metadata(path)? {
        return Ok(metadata.last_access_unix);
    }

    unix_timestamp(fs::metadata(path)?.modified()?)
}

fn current_unix_timestamp() -> Result<u64, CoreError> {
    unix_timestamp(SystemTime::now())
}

fn unix_timestamp(time: SystemTime) -> Result<u64, CoreError> {
    Ok(time
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            CoreError::Operator(format!("system clock is before unix epoch: {error}"))
        })?
        .as_secs())
}

fn payload_base_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix(".pkg.tar.zst"))
        .map(ToOwned::to_owned)
}

fn package_cache_base_name(
    pkgname: &str,
    pkgver: &str,
    pkgrel: u64,
    arch: Option<&str>,
) -> Option<String> {
    Some(format!("{pkgname}-{pkgver}-{pkgrel}-{}", arch?))
}

fn state_archive_paths(states_dir: &Path) -> Result<Vec<PathBuf>, CoreError> {
    if !states_dir.exists() {
        return Ok(Vec::new());
    }

    let mut paths = Vec::new();
    for entry in fs::read_dir(states_dir)? {
        let path = entry?.path();
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension == "json")
        {
            paths.push(path);
        }
    }
    Ok(paths)
}

fn file_size(path: &Path) -> Result<u64, CoreError> {
    Ok(fs::metadata(path)?.len())
}

fn file_size_if_present(path: &Path) -> Result<u64, CoreError> {
    if !path.exists() {
        return Ok(0);
    }
    file_size(path)
}
