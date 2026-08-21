use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use elda_recipe::SourceDefinition;

use crate::BuildError;

use super::{is_launcher_candidate, normalized_archive_path};

pub(super) fn zip_executable_candidates(
    source: &SourceDefinition,
    downloaded_path: &Path,
) -> Result<Vec<PathBuf>, BuildError> {
    let file = fs::File::open(downloaded_path)?;
    let mut archive = ::zip::ZipArchive::new(file).map_err(zip_error)?;
    let mut candidates = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(zip_error)?;
        if !entry.is_file() || entry.unix_mode().unwrap_or(0) & 0o111 == 0 {
            continue;
        }
        let path = normalized_archive_path(source, &enclosed_name(&entry)?)?;
        if is_launcher_candidate(&path) {
            candidates.push(path);
        }
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

pub(super) fn extract_zip_binary(
    downloaded_path: &Path,
    source: &SourceDefinition,
    requested_path: &Path,
    basename_only: bool,
    destination: &Path,
    matched: &mut bool,
) -> Result<(), BuildError> {
    let file = fs::File::open(downloaded_path)?;
    let mut archive = ::zip::ZipArchive::new(file).map_err(zip_error)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        if !entry.is_file() {
            continue;
        }
        let path = normalized_archive_path(source, &enclosed_name(&entry)?)?;
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
        let mut output = fs::File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
        output.flush()?;
        *matched = true;
    }
    Ok(())
}

fn enclosed_name(entry: &::zip::read::ZipFile<'_, fs::File>) -> Result<PathBuf, BuildError> {
    entry.enclosed_name().ok_or_else(|| {
        BuildError::Invalid(format!("zip member `{}` has an unsafe path", entry.name()))
    })
}

fn zip_error(error: ::zip::result::ZipError) -> BuildError {
    BuildError::Invalid(format!("invalid zip artifact: {error}"))
}
