use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::BuildError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub entries: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub kind: ManifestEntryKind,
    pub sha256: Option<String>,
    pub size: u64,
    pub mode: u32,
    pub link_target: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManifestEntryKind {
    Directory,
    RegularFile,
    Symlink,
}

pub fn collect_manifest(stage_root: &Path) -> Result<PackageManifest, BuildError> {
    let mut entries = Vec::new();
    collect_entries(stage_root, stage_root, &mut entries)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));

    Ok(PackageManifest { entries })
}

pub fn manifest_hash(manifest: &PackageManifest) -> Result<(String, Vec<u8>), BuildError> {
    let bytes = serde_json::to_vec_pretty(manifest)?;
    Ok((sha256_bytes(&bytes), bytes))
}

pub fn sha256_file(path: &Path) -> Result<String, BuildError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn archive_stage(stage_root: &Path, payload_path: &Path) -> Result<(), BuildError> {
    let file = fs::File::create(payload_path)?;
    let encoder = zstd::Encoder::new(file, 19)?;
    let mut builder = tar::Builder::new(encoder.auto_finish());

    let mut entries = fs::read_dir(stage_root)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    entries.sort();

    for path in entries {
        append_path(&mut builder, stage_root, &path)?;
    }
    builder.finish()?;

    Ok(())
}

fn collect_entries(
    stage_root: &Path,
    current: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> Result<(), BuildError> {
    let mut children = fs::read_dir(current)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    children.sort();

    for path in children {
        let metadata = fs::symlink_metadata(&path)?;
        let kind = if metadata.file_type().is_dir() {
            ManifestEntryKind::Directory
        } else if metadata.file_type().is_symlink() {
            ManifestEntryKind::Symlink
        } else {
            ManifestEntryKind::RegularFile
        };
        let relative = path
            .strip_prefix(stage_root)
            .map_err(|_| BuildError::Invalid("staged path escaped the staging root".to_owned()))?;
        let canonical_path = format!("/{}", relative.display());
        let mode = metadata.permissions().mode();

        let entry = match kind {
            ManifestEntryKind::Directory => ManifestEntry {
                path: canonical_path,
                kind,
                sha256: None,
                size: 0,
                mode,
                link_target: None,
            },
            ManifestEntryKind::Symlink => ManifestEntry {
                path: canonical_path,
                kind,
                sha256: None,
                size: 0,
                mode,
                link_target: Some(fs::read_link(&path)?.display().to_string()),
            },
            ManifestEntryKind::RegularFile => ManifestEntry {
                path: canonical_path,
                kind,
                sha256: Some(sha256_file(&path)?),
                size: metadata.len(),
                mode,
                link_target: None,
            },
        };
        entries.push(entry);

        if metadata.file_type().is_dir() {
            collect_entries(stage_root, &path, entries)?;
        }
    }

    Ok(())
}

fn append_path<W: Write>(
    builder: &mut tar::Builder<W>,
    stage_root: &Path,
    path: &Path,
) -> Result<(), BuildError> {
    let relative = path
        .strip_prefix(stage_root)
        .map_err(|_| BuildError::Invalid("staged path escaped the staging root".to_owned()))?;
    let metadata = fs::symlink_metadata(path)?;

    if metadata.is_dir() {
        builder.append_dir(relative, path)?;
        let mut children = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<PathBuf>, _>>()?;
        children.sort();
        for child in children {
            append_path(builder, stage_root, &child)?;
        }
        return Ok(());
    }

    if metadata.file_type().is_symlink() {
        builder.append_path_with_name(path, relative)?;
        return Ok(());
    }

    let mut file = fs::File::open(path)?;
    builder.append_file(relative, &mut file)?;

    Ok(())
}
