use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;

use elda_db::Database;
use elda_repo::{DEFAULT_REMOTE_CHANNEL, list_caches, load_snapshot};

use crate::cache::{ensure_cached_blob, local_cache_directory, local_payloads};
use crate::cli::{CacheCommand, Cli, Command, MirrorRemoteArgs, PushLocalArgs};
use crate::config::state_layout;
use crate::error::PopulateError;
use crate::manifest::write_manifest;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PopulateReport {
    pub(crate) mode: String,
    pub(crate) cache: String,
    pub(crate) considered: usize,
    pub(crate) mirrored: usize,
    pub(crate) manifest_path: Option<PathBuf>,
    pub(crate) summary: String,
}

pub(crate) fn run(cli: Cli) -> Result<PopulateReport, PopulateError> {
    match cli.command {
        Command::Cache { command } => {
            let layout = state_layout(cli.root, cli.prefix)?;
            match command {
                CacheCommand::PushLocal(args) => push_local(layout, args),
                CacheCommand::MirrorRemote(args) => mirror_remote(layout, args),
            }
        }
    }
}

pub(crate) fn push_local(
    layout: elda_db::StateLayout,
    args: PushLocalArgs,
) -> Result<PopulateReport, PopulateError> {
    if !args.installed {
        return Err(PopulateError::Operator(
            "cache push-local currently requires `--installed` to define the payload set"
                .to_owned(),
        ));
    }

    let database = Database::new(layout.clone());
    database.bootstrap()?;
    let cache = configured_cache(&layout, &args.cache)?;
    require_writable_mode(&cache.base_url, args.manifest_out.as_ref(), args.dry_run)?;
    let selected_packages = args.package.into_iter().collect::<BTreeSet<_>>();
    let installed = database.list_installed_packages()?;
    let digests = installed
        .into_iter()
        .filter(|record| {
            selected_packages.is_empty() || selected_packages.contains(&record.pkgname)
        })
        .filter_map(|record| record.payload_sha256.map(|digest| (record.pkgname, digest)))
        .collect::<Vec<_>>();
    if digests.is_empty() {
        return Err(PopulateError::Operator(
            "no installed payload digests matched the requested package set".to_owned(),
        ));
    }

    let payloads = local_payloads(&layout.cache_pkg_dir)?;
    let mut entries = Vec::new();
    for payload in payloads {
        let Some(package_name) = digests
            .iter()
            .find_map(|(package_name, digest)| (digest == &payload.sha256).then_some(package_name))
        else {
            continue;
        };
        entries.push(ensure_cached_blob(
            &cache,
            &payload.sha256,
            &format!("file://{}", payload.path.display()),
            package_name,
            args.dry_run,
        )?);
    }
    if entries.is_empty() {
        return Err(PopulateError::Operator(
            "installed payloads were referenced in the database, but matching local payload artifacts were not found in the package cache".to_owned(),
        ));
    }

    let manifest_path = match args.manifest_out {
        Some(path) => Some(write_manifest(&path, entries.clone())?),
        None => None,
    };

    Ok(PopulateReport {
        mode: "push-local".to_owned(),
        cache: cache.name.clone(),
        considered: digests.len(),
        mirrored: entries.len(),
        manifest_path,
        summary: populate_summary("pushed", &cache.name, entries.len(), args.dry_run),
    })
}

pub(crate) fn mirror_remote(
    layout: elda_db::StateLayout,
    args: MirrorRemoteArgs,
) -> Result<PopulateReport, PopulateError> {
    let cache = configured_cache(&layout, &args.cache)?;
    require_writable_mode(&cache.base_url, args.manifest_out.as_ref(), args.dry_run)?;
    let snapshot = load_snapshot(&layout.db_dir.join("repo-snapshot.json"))?;
    let selected = args.package.into_iter().collect::<BTreeSet<_>>();
    let selected_channel = args.channel;
    let packages = snapshot
        .packages
        .into_iter()
        .filter(|package| package.remote_name == args.remote)
        .filter(|package| {
            selected_channel
                .as_deref()
                .is_none_or(|channel| package_matches_channel(package.channel.as_deref(), channel))
        })
        .filter(|package| selected.is_empty() || selected.contains(&package.pkgname))
        .filter(|package| package.asset_url.is_some() && package.sha256.is_some())
        .collect::<Vec<_>>();
    if packages.is_empty() {
        let scope = selected_channel
            .as_deref()
            .map(|channel| format!("remote `{}` on channel `{channel}`", args.remote))
            .unwrap_or_else(|| format!("remote `{}`", args.remote));
        return Err(PopulateError::Operator(format!(
            "no binary-capable synced packages matched {scope}"
        )));
    }

    let mut entries = Vec::new();
    for package in &packages {
        let source_url = package.asset_url.as_deref().ok_or_else(|| {
            PopulateError::Operator(format!(
                "synced package `{}` is missing `asset_url`",
                package.pkgname
            ))
        })?;
        let digest = package.sha256.as_deref().ok_or_else(|| {
            PopulateError::Operator(format!(
                "synced package `{}` is missing `sha256`",
                package.pkgname
            ))
        })?;
        entries.push(ensure_cached_blob(
            &cache,
            digest,
            source_url,
            &package.pkgname,
            args.dry_run,
        )?);
    }

    let manifest_path = match args.manifest_out {
        Some(path) => Some(write_manifest(&path, entries.clone())?),
        None => None,
    };

    Ok(PopulateReport {
        mode: "mirror-remote".to_owned(),
        cache: cache.name.clone(),
        considered: packages.len(),
        mirrored: entries.len(),
        manifest_path,
        summary: populate_summary("mirrored", &cache.name, entries.len(), args.dry_run),
    })
}

fn configured_cache(
    layout: &elda_db::StateLayout,
    cache_name: &str,
) -> Result<elda_repo::CacheDocument, PopulateError> {
    list_caches(&layout.caches_dir)?
        .into_iter()
        .find(|cache| cache.enabled && cache.name == cache_name)
        .ok_or_else(|| {
            PopulateError::Operator(format!("configured cache `{cache_name}` was not found"))
        })
}

fn require_writable_mode(
    base_url: &str,
    manifest_out: Option<&PathBuf>,
    dry_run: bool,
) -> Result<(), PopulateError> {
    if dry_run || manifest_out.is_some() || local_cache_directory(base_url).is_some() {
        return Ok(());
    }

    Err(PopulateError::Operator(
        "direct cache writes currently require a local path or `file://` cache; use `--manifest-out` for HTTP/object-store cache targets in the current populate slice".to_owned(),
    ))
}

fn populate_summary(verb: &str, cache_name: &str, count: usize, dry_run: bool) -> String {
    let mode = if dry_run { "would have " } else { "" };
    format!("{mode}{verb} {count} payload(s) for cache `{cache_name}`.")
}

fn package_matches_channel(package_channel: Option<&str>, selected_channel: &str) -> bool {
    match package_channel {
        Some(channel) => channel == selected_channel,
        None => selected_channel == DEFAULT_REMOTE_CHANNEL,
    }
}

#[allow(dead_code)]
fn cache_supports_direct_write(base_url: &str) -> bool {
    local_cache_directory(base_url).is_some()
}
