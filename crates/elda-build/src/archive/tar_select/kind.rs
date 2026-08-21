use std::fs;
use std::io::Read;
use std::path::Path;

use elda_recipe::{ScalarValue, SourceDefinition};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveKind {
    Tar,
    TarGz,
    TarBz2,
    TarZst,
    TarXz,
    Zip,
}

pub(crate) fn infer_archive_kind(
    downloaded_path: &Path,
    source_url: &str,
    source: &SourceDefinition,
) -> Option<ArchiveKind> {
    if let Some(kind) = archive_kind_from_magic(downloaded_path) {
        return Some(kind);
    }
    if let Some(name) = downloaded_path.file_name().and_then(|name| name.to_str())
        && let Some(kind) = archive_kind_from_name(name)
    {
        return Some(kind);
    }
    if let Some(segment) = source_url.rsplit('/').next() {
        let base = segment.split(['?', '#']).next().unwrap_or(segment);
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

fn archive_kind_from_magic(path: &Path) -> Option<ArchiveKind> {
    let mut file = fs::File::open(path).ok()?;
    let mut magic = [0u8; 262];
    let read = file.read(&mut magic).ok()?;
    let head = &magic[..read];
    if head.starts_with(&[0x1f, 0x8b]) {
        Some(ArchiveKind::TarGz)
    } else if head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        Some(ArchiveKind::TarXz)
    } else if head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Some(ArchiveKind::TarZst)
    } else if head.starts_with(b"BZh") {
        Some(ArchiveKind::TarBz2)
    } else if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Some(ArchiveKind::Zip)
    } else if read >= 262 && &head[257..262] == b"ustar" {
        Some(ArchiveKind::Tar)
    } else {
        None
    }
}

fn archive_kind_from_name(name: &str) -> Option<ArchiveKind> {
    if name.ends_with(".tar") {
        Some(ArchiveKind::Tar)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        Some(ArchiveKind::TarGz)
    } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
        Some(ArchiveKind::TarBz2)
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") {
        Some(ArchiveKind::TarZst)
    } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
        Some(ArchiveKind::TarXz)
    } else if name.ends_with(".zip") {
        Some(ArchiveKind::Zip)
    } else {
        None
    }
}

fn string_field_optional<'a>(source: &'a SourceDefinition, key: &str) -> Option<&'a str> {
    match source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}
