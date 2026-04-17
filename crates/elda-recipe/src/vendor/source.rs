use std::path::Path;

use crate::error::RecipeError;

use super::github::{
    detect_release_asset, fetch_github_release, fetch_sha256, parse_github_release_spec,
};
use super::model::{GitHubReleaseSpec, ParsedVendorManifestLine, ResolvedVendorSource};

pub(super) fn resolve_vendor_source(
    source: &str,
    binary: Option<&str>,
    asset: Option<&str>,
    package_name: &str,
) -> Result<ResolvedVendorSource, RecipeError> {
    if let Some(spec) = parse_github_release_spec(source) {
        return resolve_github_release(spec, binary, asset, package_name);
    }

    resolve_archive_source(source, binary, asset, package_name)
}

pub(super) fn parse_vendor_manifest_line(
    line: &str,
) -> Result<ParsedVendorManifestLine, RecipeError> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return Err(RecipeError::InvalidInput(format!(
            "vendor manifest line must contain `<pkgname> <source>`: {line}"
        )));
    }

    let package_name = tokens[0].to_owned();
    let source = tokens[1].to_owned();
    let mut binary = None;
    let mut asset = None;
    let mut index = 2;
    while index < tokens.len() {
        match tokens[index] {
            "--binary" => {
                let value = tokens.get(index + 1).ok_or_else(|| {
                    RecipeError::InvalidInput("`--binary` requires a value".to_owned())
                })?;
                binary = Some((*value).to_owned());
                index += 2;
            }
            "--asset" => {
                let value = tokens.get(index + 1).ok_or_else(|| {
                    RecipeError::InvalidInput("`--asset` requires a value".to_owned())
                })?;
                asset = Some((*value).to_owned());
                index += 2;
            }
            other => {
                return Err(RecipeError::InvalidInput(format!(
                    "unsupported vendor manifest token `{other}`"
                )));
            }
        }
    }

    Ok(ParsedVendorManifestLine {
        package_name,
        source,
        binary,
        asset,
    })
}

fn resolve_archive_source(
    source: &str,
    binary: Option<&str>,
    asset: Option<&str>,
    package_name: &str,
) -> Result<ResolvedVendorSource, RecipeError> {
    let url = normalize_source_url(source)?;
    let asset_name = asset
        .map(ToOwned::to_owned)
        .or_else(|| url.rsplit('/').next().map(ToOwned::to_owned))
        .ok_or_else(|| {
            RecipeError::InvalidInput(
                "could not infer an asset name from the vendor source".to_owned(),
            )
        })?;
    let sha256 = fetch_sha256(&url)?;
    let rename = Some(package_name.to_owned());

    if looks_like_tar_archive(&asset_name) && binary.is_none() {
        return Err(RecipeError::InvalidInput(
            "archive vendor sources require `--binary <name>` in the current slice".to_owned(),
        ));
    }

    Ok(ResolvedVendorSource::UrlArchive {
        url,
        sha256,
        binary: binary.map(ToOwned::to_owned),
        rename,
    })
}

fn resolve_github_release(
    spec: GitHubReleaseSpec<'_>,
    binary: Option<&str>,
    asset: Option<&str>,
    package_name: &str,
) -> Result<ResolvedVendorSource, RecipeError> {
    let release = fetch_github_release(spec.repo, spec.tag_or_latest)?;
    let selected_asset = match asset {
        Some(asset_name) => release
            .assets
            .iter()
            .find(|candidate| candidate.name == asset_name)
            .cloned()
            .ok_or_else(|| {
                RecipeError::InvalidInput(format!(
                    "release `{}` does not contain asset `{asset_name}`",
                    release.tag_name
                ))
            })?,
        None => detect_release_asset(&release)?,
    };
    let sha256 = fetch_sha256(&selected_asset.browser_download_url)?;
    let rename = Some(package_name.to_owned());

    if looks_like_tar_archive(&selected_asset.name) && binary.is_none() {
        return Err(RecipeError::InvalidInput(
            "archive vendor sources require `--binary <name>` in the current slice".to_owned(),
        ));
    }

    Ok(ResolvedVendorSource::GitHubRelease {
        repo: spec.repo.to_owned(),
        tag: release.tag_name,
        asset: selected_asset.name,
        sha256,
        binary: binary.map(ToOwned::to_owned),
        rename,
    })
}

fn normalize_source_url(source: &str) -> Result<String, RecipeError> {
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("file://")
    {
        return Ok(source.to_owned());
    }

    let local_path = Path::new(source);
    if local_path.exists() {
        return Ok(format!("file://{}", local_path.display()));
    }

    Err(RecipeError::InvalidInput(format!(
        "unsupported vendor source `{source}`"
    )))
}

fn looks_like_tar_archive(name: &str) -> bool {
    [
        ".tar", ".tar.gz", ".tgz", ".tar.xz", ".txz", ".tar.zst", ".tzst",
    ]
    .iter()
    .any(|suffix| name.ends_with(suffix))
}
