//! Survey a local artifact file the operator handed to Elda directly.
//!
//! The file is identified by **content magic, not by extension**, then listed
//! read-only. Nothing is executed and nothing is extracted: the survey exists so
//! the operator can see what a recipe would install before one is written.

mod classify;
mod identity;

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Component;
use std::path::{Path, PathBuf};

use bzip2::read::BzDecoder;
use elda_types::{ArtifactEntry, ArtifactEntryKind, ArtifactFormat, ArtifactSurvey};
use flate2::read::GzDecoder;
use liblzma::read::XzDecoder;
use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::BuildError;
use crate::manifest::sha256_file;
use classify::classify;
use identity::{infer_architecture, infer_identity};

#[cfg(test)]
use identity::{elf_architecture, looks_like_version, split_name_version};

const MAX_ARCHIVE_ENTRIES: usize = 100_000;
const MAX_ARCHIVE_MEMBER_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_UNPACKED_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const MAX_ARCHIVE_PATH_BYTES: usize = 4096;

/// Identify an artifact by leading magic bytes.
///
/// Returns `None` for anything Elda has no reader for, so callers can fall
/// through to the existing source-tree import path rather than guessing.
#[must_use]
pub fn identify(path: &Path) -> Option<ArtifactFormat> {
    let mut file = File::open(path).ok()?;
    let mut magic = [0u8; 262];
    let read = read_up_to(&mut file, &mut magic).ok()?;
    let head = &magic[..read];

    if head.starts_with(&[0x1f, 0x8b]) {
        return Some(ArtifactFormat::TarGz);
    }
    if head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        return Some(ArtifactFormat::TarXz);
    }
    if head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        return Some(ArtifactFormat::TarZst);
    }
    if head.starts_with(b"BZh") {
        return Some(ArtifactFormat::TarBz2);
    }
    if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        return Some(ArtifactFormat::Zip);
    }
    if head.starts_with(b"\x7fELF") {
        // An AppImage is an ELF with an appended filesystem, so the ELF answer
        // alone would misfile every AppImage as a plain binary.
        return Some(appimage_or_elf(path, head));
    }
    // POSIX tar keeps `ustar` at offset 257.
    if read >= 262 && &head[257..262] == b"ustar" {
        return Some(ArtifactFormat::Tar);
    }
    None
}

/// Distinguish an AppImage from a plain ELF binary.
///
/// Marked images carry `AI\x01` / `AI\x02` at bytes 8..11 and cost nothing to
/// spot. Unmarked runtimes (the `uruntime` / RunImage family) only reveal
/// themselves by carrying a payload magic at a computed offset, so those pay for
/// a bounded scan — which is still read-only and never executes the image.
fn appimage_or_elf(path: &Path, head: &[u8]) -> ArtifactFormat {
    if elda_appimage::appimage_type_magic(head).is_some() {
        return ArtifactFormat::AppImage;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return ArtifactFormat::Elf;
    };
    if elda_appimage::payload_location(&bytes).is_ok() {
        return ArtifactFormat::AppImage;
    }
    ArtifactFormat::Elf
}

fn read_up_to(file: &mut File, buffer: &mut [u8]) -> std::io::Result<usize> {
    file.seek(SeekFrom::Start(0))?;
    let mut filled = 0;
    while filled < buffer.len() {
        match file.read(&mut buffer[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

/// Inspect a local artifact and describe what installing it would mean.
pub fn survey(path: &Path) -> Result<ArtifactSurvey, BuildError> {
    let format = identify(path).ok_or_else(|| {
        BuildError::Unsupported(format!(
            "`{}` is not a recognised artifact; expected a tar/zip archive or an ELF binary",
            path.display()
        ))
    })?;

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let size = std::fs::metadata(path)?.len();
    let sha256 = sha256_file(path)?;
    let source_path = canonical_artifact_path(path)?.display().to_string();

    let entries = if format.is_archive() {
        list_archive(path, format)?
    } else {
        vec![ArtifactEntry {
            path: file_name.clone(),
            kind: ArtifactEntryKind::Executable,
            size,
            executable: true,
        }]
    };

    let strip_components = common_prefix_depth(&entries);
    let root = archive_root(&entries);
    let (name, version) = infer_identity(root.as_deref(), &file_name);
    let architecture = infer_architecture(path, format, root.as_deref(), &file_name)?;
    let appimage_payload = if format == ArtifactFormat::AppImage {
        Some(validate_appimage(path)?)
    } else {
        None
    };

    Ok(ArtifactSurvey {
        source_path,
        file_name,
        format,
        sha256,
        size,
        strip_components,
        name,
        version,
        architecture,
        appimage_payload,
        entries,
    })
}

fn list_archive(path: &Path, format: ArtifactFormat) -> Result<Vec<ArtifactEntry>, BuildError> {
    let file = BufReader::new(File::open(path)?);
    match format {
        ArtifactFormat::Tar => collect(Archive::new(file)),
        ArtifactFormat::TarGz => collect(Archive::new(GzDecoder::new(file))),
        ArtifactFormat::TarXz => collect(Archive::new(XzDecoder::new(file))),
        ArtifactFormat::TarZst => collect(Archive::new(ZstdDecoder::new(file)?)),
        ArtifactFormat::TarBz2 => collect(Archive::new(BzDecoder::new(file))),
        ArtifactFormat::Zip => collect_zip(path),
        ArtifactFormat::Elf | ArtifactFormat::AppImage => Ok(Vec::new()),
    }
}

fn collect<R: Read>(mut archive: Archive<R>) -> Result<Vec<ArtifactEntry>, BuildError> {
    let mut entries = Vec::new();
    let mut unpacked_size = 0;
    for entry in archive.entries()? {
        let entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = checked_archive_path(entry.path()?.as_ref())?;
        let executable = entry.header().mode().unwrap_or(0) & 0o111 != 0;
        let size = entry.header().size().unwrap_or(0);
        push_entry(&mut entries, &mut unpacked_size, path, size, executable)?;
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn collect_zip(path: &Path) -> Result<Vec<ArtifactEntry>, BuildError> {
    let file = File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_error)?;
    let mut entries = Vec::new();
    let mut unpacked_size = 0;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(zip_error)?;
        if !entry.is_file() {
            continue;
        }
        let path = entry.enclosed_name().ok_or_else(|| {
            BuildError::Invalid(format!("zip member `{}` has an unsafe path", entry.name()))
        })?;
        let path = checked_archive_path(&path)?;
        let executable = entry.unix_mode().unwrap_or(0) & 0o111 != 0;
        push_entry(
            &mut entries,
            &mut unpacked_size,
            path,
            entry.size(),
            executable,
        )?;
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn push_entry(
    entries: &mut Vec<ArtifactEntry>,
    unpacked_size: &mut u64,
    path: String,
    size: u64,
    executable: bool,
) -> Result<(), BuildError> {
    if entries.len() >= MAX_ARCHIVE_ENTRIES {
        return Err(BuildError::Invalid(format!(
            "artifact contains more than {MAX_ARCHIVE_ENTRIES} regular files"
        )));
    }
    if size > MAX_ARCHIVE_MEMBER_BYTES {
        return Err(BuildError::Invalid(format!(
            "artifact member `{path}` exceeds the {} byte survey limit",
            MAX_ARCHIVE_MEMBER_BYTES
        )));
    }
    let unpacked = unpacked_size
        .checked_add(size)
        .ok_or_else(|| BuildError::Invalid("artifact member sizes overflow u64".to_owned()))?;
    if unpacked > MAX_ARCHIVE_UNPACKED_BYTES {
        return Err(BuildError::Invalid(format!(
            "artifact declares more than {} bytes of unpacked regular files",
            MAX_ARCHIVE_UNPACKED_BYTES
        )));
    }
    *unpacked_size = unpacked;
    let kind = classify(&path, executable);
    entries.push(ArtifactEntry {
        path,
        kind,
        size,
        executable,
    });
    Ok(())
}

fn checked_archive_path(path: &Path) -> Result<String, BuildError> {
    if path.as_os_str().as_encoded_bytes().len() > MAX_ARCHIVE_PATH_BYTES {
        return Err(BuildError::Invalid(format!(
            "artifact member path exceeds {MAX_ARCHIVE_PATH_BYTES} bytes"
        )));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(BuildError::Invalid(format!(
            "artifact member `{}` has an unsafe path",
            path.display()
        )));
    }
    path.to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| BuildError::Invalid("artifact member path is not valid UTF-8".to_owned()))
}

fn zip_error(error: zip::result::ZipError) -> BuildError {
    BuildError::Invalid(format!("invalid zip artifact: {error}"))
}

/// Number of leading path components every member shares.
fn common_prefix_depth(entries: &[ArtifactEntry]) -> u32 {
    let mut roots = entries
        .iter()
        .filter(|entry| !entry.kind.is_droppable())
        .filter_map(|entry| entry.path.split('/').next())
        .filter(|segment| !segment.is_empty());

    let Some(first) = roots.next() else {
        return 0;
    };
    if roots.any(|segment| segment != first) {
        return 0;
    }
    // A single shared root is only strippable when it is a directory, i.e. some
    // member actually lives beneath it.
    u32::from(
        entries
            .iter()
            .filter(|entry| !entry.kind.is_droppable())
            .any(|entry| entry.path.contains('/')),
    )
}

fn archive_root(entries: &[ArtifactEntry]) -> Option<String> {
    if common_prefix_depth(entries) == 0 {
        return None;
    }
    entries
        .iter()
        .find(|entry| !entry.kind.is_droppable())
        .and_then(|entry| entry.path.split('/').next())
        .map(ToOwned::to_owned)
}

fn validate_appimage(path: &Path) -> Result<String, BuildError> {
    let bytes = std::fs::read(path)?;
    let location = elda_appimage::payload_location(&bytes).map_err(|error| {
        BuildError::Invalid(format!(
            "`{}` is not a supported AppImage: {error}",
            path.display()
        ))
    })?;
    Ok(location.format.label().to_owned())
}

/// Absolute path helper for callers that accept operator-supplied relative paths.
pub fn canonical_artifact_path(path: &Path) -> Result<PathBuf, BuildError> {
    std::fs::canonicalize(path).map_err(BuildError::from)
}

#[cfg(test)]
mod tests;
