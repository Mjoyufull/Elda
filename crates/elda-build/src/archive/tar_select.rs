use std::fs;
use std::io::BufReader;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use liblzma::read::XzDecoder;
use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

use elda_recipe::{ScalarValue, SourceDefinition};

use crate::error::BuildError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ArchiveKind {
    Tar,
    TarGz,
    TarZst,
    TarXz,
}

pub(super) fn stage_binary_from_tar(
    source: &SourceDefinition,
    downloaded_path: &Path,
    bin_dir: &Path,
    kind: ArchiveKind,
) -> Result<(), BuildError> {
    let requested_binary = match string_field_optional(source, "binary") {
        Some(binary) => binary.to_owned(),
        None => infer_archive_binary(downloaded_path, kind)?,
    };
    let install_name = string_field_optional(source, "rename")
        .map(ToOwned::to_owned)
        .or_else(|| plain_file_name(&requested_binary))
        .ok_or_else(|| {
            BuildError::Invalid(format!(
                "binary source `{}` requires a valid `binary` path",
                source.kind
            ))
        })?;
    let destination = bin_dir.join(install_name);
    let requested_path = Path::new(&requested_binary);
    let basename_only = !requested_binary.contains('/');
    let mut matched = false;

    match kind {
        ArchiveKind::Tar => {
            let file = fs::File::open(downloaded_path)?;
            extract_tar_binary(
                Archive::new(BufReader::new(file)),
                requested_path,
                basename_only,
                &destination,
                &mut matched,
            )?;
        }
        ArchiveKind::TarGz => {
            let file = fs::File::open(downloaded_path)?;
            extract_tar_binary(
                Archive::new(GzDecoder::new(BufReader::new(file))),
                requested_path,
                basename_only,
                &destination,
                &mut matched,
            )?;
        }
        ArchiveKind::TarZst => {
            let file = fs::File::open(downloaded_path)?;
            let decoder = ZstdDecoder::new(BufReader::new(file))?;
            extract_tar_binary(
                Archive::new(decoder),
                requested_path,
                basename_only,
                &destination,
                &mut matched,
            )?;
        }
        ArchiveKind::TarXz => {
            let file = fs::File::open(downloaded_path)?;
            extract_tar_binary(
                Archive::new(XzDecoder::new(BufReader::new(file))),
                requested_path,
                basename_only,
                &destination,
                &mut matched,
            )?;
        }
    }

    if !matched {
        return Err(BuildError::Invalid(format!(
            "archive `{}` does not contain requested binary `{requested_binary}`",
            downloaded_path.display()
        )));
    }

    fs::set_permissions(&destination, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

pub(super) fn infer_archive_kind(
    downloaded_path: &Path,
    source_url: &str,
    source: &SourceDefinition,
) -> Option<ArchiveKind> {
    if let Some(name) = downloaded_path.file_name().and_then(|n| n.to_str())
        && let Some(kind) = archive_kind_from_name(name)
    {
        return Some(kind);
    }

    if let Some(segment) = source_url.rsplit('/').next() {
        let base = segment.split('?').next().unwrap_or(segment);
        if let Some(kind) = archive_kind_from_name(base) {
            return Some(kind);
        }
    }

    if let Some(asset) = string_field_optional(source, "asset")
        && let Some(kind) = archive_kind_from_name(asset)
    {
        return Some(kind);
    }

    None
}

fn infer_archive_binary(downloaded_path: &Path, kind: ArchiveKind) -> Result<String, BuildError> {
    let candidates = match kind {
        ArchiveKind::Tar => {
            let file = fs::File::open(downloaded_path)?;
            executable_candidates(Archive::new(BufReader::new(file)))?
        }
        ArchiveKind::TarGz => {
            let file = fs::File::open(downloaded_path)?;
            executable_candidates(Archive::new(GzDecoder::new(BufReader::new(file))))?
        }
        ArchiveKind::TarZst => {
            let file = fs::File::open(downloaded_path)?;
            let decoder = ZstdDecoder::new(BufReader::new(file))?;
            executable_candidates(Archive::new(decoder))?
        }
        ArchiveKind::TarXz => {
            let file = fs::File::open(downloaded_path)?;
            executable_candidates(Archive::new(XzDecoder::new(BufReader::new(file))))?
        }
    };

    match candidates.as_slice() {
        [candidate] => Ok(candidate.display().to_string()),
        [] => Err(BuildError::Invalid(
            "binary archive does not declare `binary` and no executable candidate was found"
                .to_owned(),
        )),
        many => Err(BuildError::Invalid(format!(
            "binary archive does not declare `binary` and contains multiple executable candidates: {}",
            many.iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn executable_candidates<R: std::io::Read>(
    mut archive: Archive<R>,
) -> Result<Vec<PathBuf>, BuildError> {
    let mut candidates = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        if entry.header().mode().unwrap_or(0) & 0o111 == 0 {
            continue;
        }

        let path = entry.path()?.into_owned();
        if is_launcher_candidate(&path) {
            candidates.push(path);
        }
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn is_launcher_candidate(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && name != "."
        && name != ".."
        && !name.starts_with("lib")
        && !name.contains(".so")
}

fn extract_tar_binary<R: std::io::Read>(
    mut archive: Archive<R>,
    requested_path: &Path,
    basename_only: bool,
    destination: &Path,
    matched: &mut bool,
) -> Result<(), BuildError> {
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }

        let path = entry.path()?.into_owned();
        let is_match = if basename_only {
            path.file_name() == requested_path.file_name()
        } else {
            path == requested_path
        };
        if !is_match {
            continue;
        }

        if *matched {
            if destination.exists() {
                fs::remove_file(destination)?;
            }
            return Err(BuildError::Invalid(format!(
                "archive contains multiple matches for `{}`; use an explicit binary path",
                requested_path.display()
            )));
        }

        entry.unpack(destination)?;
        *matched = true;
    }

    Ok(())
}

fn archive_kind_from_name(name: &str) -> Option<ArchiveKind> {
    if name.ends_with(".tar") {
        Some(ArchiveKind::Tar)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(ArchiveKind::TarGz)
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        Some(ArchiveKind::TarZst)
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        Some(ArchiveKind::TarXz)
    } else {
        None
    }
}

fn plain_file_name(value: &str) -> Option<String> {
    Path::new(value)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn string_field_optional<'a>(source: &'a SourceDefinition, key: &str) -> Option<&'a str> {
    match source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
