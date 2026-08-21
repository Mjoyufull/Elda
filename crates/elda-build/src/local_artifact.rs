//! Survey a local artifact file the operator handed to Elda directly.
//!
//! The file is identified by **content magic, not by extension**, then listed
//! read-only. Nothing is executed and nothing is extracted: the survey exists so
//! the operator can see what a recipe would install before one is written.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use elda_types::{ArtifactEntry, ArtifactEntryKind, ArtifactFormat, ArtifactSurvey};
use flate2::read::GzDecoder;
use liblzma::read::XzDecoder;
use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::BuildError;
use crate::manifest::sha256_file;

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

    Ok(ArtifactSurvey {
        file_name,
        format,
        sha256,
        size,
        strip_components,
        name,
        version,
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
        ArtifactFormat::TarBz2 | ArtifactFormat::Zip => Err(BuildError::Unsupported(format!(
            "`{}` artifacts are recognised but not yet surveyable",
            format.label()
        ))),
        ArtifactFormat::Elf | ArtifactFormat::AppImage => Ok(Vec::new()),
    }
}

fn collect<R: Read>(mut archive: Archive<R>) -> Result<Vec<ArtifactEntry>, BuildError> {
    let mut entries = Vec::new();
    for entry in archive.entries()? {
        let entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.display().to_string();
        let executable = entry.header().mode().unwrap_or(0) & 0o111 != 0;
        let size = entry.header().size().unwrap_or(0);
        let kind = classify(&path, executable);
        entries.push(ArtifactEntry {
            path,
            kind,
            size,
            executable,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

/// Decide what a member is, so staging can place it without asking.
fn classify(path: &str, executable: bool) -> ArtifactEntryKind {
    let lower = path.to_ascii_lowercase();
    let file_name = lower.rsplit('/').next().unwrap_or(&lower).to_owned();

    if matches!(
        extension(&file_name).as_deref(),
        Some("exe" | "dll" | "ps1" | "bat" | "cmd" | "dylib" | "msi")
    ) {
        return ArtifactEntryKind::ForeignPlatform;
    }
    if file_name.starts_with("lib") && lower.contains(".so") {
        return ArtifactEntryKind::Library;
    }
    if lower.contains("/man/") || is_man_page(&file_name) {
        return ArtifactEntryKind::ManPage;
    }
    if is_completion(&lower, &file_name) {
        return ArtifactEntryKind::Completion;
    }
    if file_name.ends_with(".desktop") {
        return ArtifactEntryKind::Desktop;
    }
    if matches!(
        extension(&file_name).as_deref(),
        Some("png" | "svg" | "xpm")
    ) {
        return ArtifactEntryKind::Icon;
    }
    if file_name.ends_with(".metainfo.xml") || file_name.ends_with(".appdata.xml") {
        return ArtifactEntryKind::Metainfo;
    }
    if is_license(&file_name) {
        return ArtifactEntryKind::License;
    }
    if is_doc(&lower, &file_name) {
        return ArtifactEntryKind::Doc;
    }
    if executable {
        return ArtifactEntryKind::Executable;
    }
    ArtifactEntryKind::Other
}

fn extension(file_name: &str) -> Option<String> {
    file_name.rsplit_once('.').map(|(_, ext)| ext.to_owned())
}

fn is_man_page(file_name: &str) -> bool {
    let stem = file_name.strip_suffix(".gz").unwrap_or(file_name);
    stem.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.len() == 1 && ext.chars().all(|c| c.is_ascii_digit()))
}

fn is_completion(lower: &str, file_name: &str) -> bool {
    if lower.contains("completion") || lower.contains("/complete/") {
        return true;
    }
    matches!(
        extension(file_name).as_deref(),
        Some("bash" | "zsh" | "fish")
    ) || file_name.starts_with('_')
}

fn is_license(file_name: &str) -> bool {
    let stem = file_name.split('.').next().unwrap_or(file_name);
    matches!(
        stem,
        "license" | "licence" | "copying" | "unlicense" | "notice"
    )
}

fn is_doc(lower: &str, file_name: &str) -> bool {
    if lower.contains("/doc/") || lower.contains("/docs/") {
        return true;
    }
    let stem = file_name.split('.').next().unwrap_or(file_name);
    matches!(
        stem,
        "readme" | "changelog" | "changes" | "authors" | "contributing"
    ) || extension(file_name).as_deref() == Some("md")
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
    u32::from(entries.iter().any(|entry| entry.path.contains('/')))
}

fn archive_root(entries: &[ArtifactEntry]) -> Option<String> {
    if common_prefix_depth(entries) == 0 {
        return None;
    }
    entries
        .first()
        .and_then(|entry| entry.path.split('/').next())
        .map(ToOwned::to_owned)
}

/// Infer `(name, version)` from the archive root, falling back to the file name.
///
/// `delta-0.18.2-x86_64-unknown-linux-gnu` yields `("delta", "0.18.2")`;
/// `delta-linux-x86_64.tar.gz` yields `("delta", None)`.
fn infer_identity(root: Option<&str>, file_name: &str) -> (Option<String>, Option<String>) {
    if let Some(root) = root
        && let (Some(name), version) = split_name_version(root)
    {
        return (Some(name), version);
    }
    let stem = strip_archive_suffix(file_name);
    let (name, version) = split_name_version(&stem);
    (name, version)
}

fn strip_archive_suffix(file_name: &str) -> String {
    let mut stem = file_name;
    for suffix in [
        ".tar.gz", ".tar.xz", ".tar.zst", ".tar.bz2", ".tgz", ".txz", ".tar", ".zip",
    ] {
        if let Some(trimmed) = stem.strip_suffix(suffix) {
            stem = trimmed;
            break;
        }
    }
    stem.to_owned()
}

/// Split on the first hyphen-delimited segment that looks like a version.
fn split_name_version(stem: &str) -> (Option<String>, Option<String>) {
    let segments: Vec<&str> = stem.split('-').collect();
    for (index, segment) in segments.iter().enumerate() {
        if index == 0 {
            continue;
        }
        if looks_like_version(segment) {
            let name = segments[..index].join("-");
            if name.is_empty() {
                break;
            }
            return (Some(name), Some((*segment).to_owned()));
        }
    }
    let name = segments
        .first()
        .filter(|segment| !segment.is_empty())
        .map(|segment| (*segment).to_owned());
    (name, None)
}

fn looks_like_version(segment: &str) -> bool {
    let candidate = segment.strip_prefix('v').unwrap_or(segment);
    let mut chars = candidate.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }
    candidate.contains('.') && candidate.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Absolute path helper for callers that accept operator-supplied relative paths.
#[must_use]
pub fn canonical_artifact_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests;
