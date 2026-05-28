use std::env::consts;
use std::fs;
use std::io::Read;

use sha2::{Digest, Sha256};

use crate::error::RecipeError;

use super::model::{GitHubReleaseAsset, GitHubReleaseResponse, GitHubReleaseSpec};

pub(super) fn fetch_github_release(
    repo: &str,
    tag_or_latest: &str,
) -> Result<GitHubReleaseResponse, RecipeError> {
    let api_url = if tag_or_latest == "latest" {
        format!("https://api.github.com/repos/{repo}/releases/latest")
    } else {
        format!("https://api.github.com/repos/{repo}/releases/tags/{tag_or_latest}")
    };
    let response = ureq::get(&api_url)
        .set("User-Agent", "elda")
        .call()
        .map_err(|error| {
            RecipeError::InvalidInput(format!("github release lookup failed: {error}"))
        })?;
    serde_json::from_reader(response.into_reader()).map_err(RecipeError::from)
}

pub(super) fn fetch_sha256(source_url: &str) -> Result<String, RecipeError> {
    if let Some(path) = source_url.strip_prefix("file://") {
        return hash_reader(fs::File::open(path)?);
    }

    let response = ureq::get(source_url)
        .set("User-Agent", "elda")
        .call()
        .map_err(|error| RecipeError::InvalidInput(format!("vendor fetch failed: {error}")))?;
    hash_reader(response.into_reader())
}

pub(super) fn parse_github_release_spec(input: &str) -> Option<GitHubReleaseSpec<'_>> {
    if input.contains("://") || input.starts_with("git@") {
        return None;
    }
    let (repo, tag_or_latest) = input.rsplit_once('@')?;
    if repo.split('/').count() != 2 || tag_or_latest.is_empty() {
        return None;
    }

    Some(GitHubReleaseSpec {
        repo,
        tag_or_latest,
    })
}

pub(super) fn detect_release_asset(
    release: &GitHubReleaseResponse,
) -> Result<GitHubReleaseAsset, RecipeError> {
    let payload_assets = release
        .assets
        .iter()
        .filter(|asset| is_payload_asset(&asset.name))
        .cloned()
        .collect::<Vec<_>>();

    if payload_assets.len() == 1 {
        return Ok(payload_assets[0].clone());
    }

    let scored = payload_assets
        .iter()
        .filter_map(|asset| score_release_asset(asset).map(|score| (asset.clone(), score)))
        .collect::<Vec<_>>();

    if scored.is_empty() {
        let asset_count = payload_assets.len().max(release.assets.len());
        return Err(RecipeError::InvalidInput(format!(
            "release `{}` has {asset_count} assets, and none matched the current platform `{}`; pass `--asset <name>`",
            release.tag_name,
            current_platform_label(),
        )));
    }

    let best_score = scored.iter().map(|(_, score)| *score).max().unwrap_or(0);
    let best = scored
        .into_iter()
        .filter(|(_, score)| *score == best_score)
        .map(|(asset, _)| asset)
        .collect::<Vec<_>>();

    if best.len() == 1 {
        return Ok(best[0].clone());
    }

    let names = best
        .iter()
        .map(|asset| asset.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    Err(RecipeError::InvalidInput(format!(
        "release `{}` has multiple assets matching the current platform `{}` ({names}); pass `--asset <name>`",
        release.tag_name,
        current_platform_label(),
    )))
}

pub(super) fn current_os_aliases() -> &'static [&'static str] {
    match consts::OS {
        "linux" => &["linux", "unknown-linux"],
        "macos" => &["darwin", "apple-darwin", "macos", "osx"],
        "freebsd" => &["freebsd", "unknown-freebsd"],
        "windows" => &["windows", "pc-windows", "win64", "win32"],
        _ => &[consts::OS],
    }
}

pub(super) fn current_arch_aliases() -> &'static [&'static str] {
    match consts::ARCH {
        "x86_64" => &["x86_64", "amd64"],
        "aarch64" => &["aarch64", "arm64"],
        "x86" => &["i686", "i386", "386"],
        "arm" => &["armv7", "armv7l", "armhf"],
        "riscv64" => &["riscv64"],
        "powerpc64" => &["ppc64le", "powerpc64le"],
        _ => &[consts::ARCH],
    }
}

fn hash_reader(mut reader: impl Read) -> Result<String, RecipeError> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn is_payload_asset(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ![
        ".sha256",
        ".sha256sum",
        ".sha512",
        ".sig",
        ".asc",
        ".minisig",
        ".sum",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn score_release_asset(asset: &GitHubReleaseAsset) -> Option<u8> {
    let lower = asset.name.to_ascii_lowercase();
    let os_match = current_os_aliases()
        .iter()
        .any(|alias| lower.contains(alias));
    let arch_match = current_arch_aliases()
        .iter()
        .any(|alias| lower.contains(alias));
    if !os_match || !arch_match {
        return None;
    }

    let abi_match = current_abi_aliases()
        .iter()
        .any(|alias| lower.contains(alias));
    Some(2 + u8::from(abi_match))
}

fn current_platform_label() -> String {
    let abi = current_abi_aliases().first().copied().unwrap_or("native");
    format!("{}-{}-{abi}", consts::OS, consts::ARCH)
}

fn current_abi_aliases() -> &'static [&'static str] {
    if cfg!(target_env = "gnu") {
        &["gnu", "glibc"]
    } else if cfg!(target_env = "musl") {
        &["musl"]
    } else if cfg!(target_env = "msvc") {
        &["msvc"]
    } else {
        &[]
    }
}
