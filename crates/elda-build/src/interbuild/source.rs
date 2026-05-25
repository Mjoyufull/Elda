use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use liblzma::read::XzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::error::BuildError;
use crate::git::ensure_git_protocol_allowed;
use crate::process::{emit_build_line, run_command};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterbuildSource {
    Git { url: String },
    Archive { url: String, sha256: Option<String> },
}

pub fn materialize_or_use_checkout(
    kind: &str,
    checkout_dir: &Path,
    upstream: Option<&InterbuildSource>,
    work_root: &Path,
    offline: bool,
    allowed_git_protocols: &[String],
    line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<PathBuf, BuildError> {
    if has_build_marker(checkout_dir) {
        return Ok(checkout_dir.to_path_buf());
    }

    let Some(upstream) = upstream else {
        return Ok(checkout_dir.to_path_buf());
    };

    materialize_upstream(
        kind,
        upstream,
        work_root,
        offline,
        allowed_git_protocols,
        line_hook,
    )
}

fn materialize_upstream(
    kind: &str,
    upstream: &InterbuildSource,
    work_root: &Path,
    offline: bool,
    allowed_git_protocols: &[String],
    line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<PathBuf, BuildError> {
    let source_root = work_root.join(format!("{kind}-upstream"));
    match upstream {
        InterbuildSource::Git { url } => {
            clone_git_source(url, &source_root, offline, allowed_git_protocols, line_hook)
        }
        InterbuildSource::Archive { url, sha256 } => {
            emit_build_line(&line_hook, format!("[Interbuild] fetching archive {url}"));
            extract_archive_source(
                kind,
                url,
                sha256.as_deref(),
                work_root,
                &source_root,
                offline,
            )
        }
    }
}

fn clone_git_source(
    url: &str,
    destination: &Path,
    offline: bool,
    allowed_git_protocols: &[String],
    line_hook: Option<std::sync::Arc<dyn Fn(&str) + Send + Sync>>,
) -> Result<PathBuf, BuildError> {
    ensure_git_protocol_allowed(url, allowed_git_protocols)?;
    if offline && !is_local_location(url) {
        return Err(BuildError::Unsupported(format!(
            "offline mode cannot fetch interbuild upstream git source `{url}`"
        )));
    }

    let mut command = Command::new("git");
    command.args(["clone", "--depth", "1", url]);
    command.arg(destination);
    emit_build_line(&line_hook, format!("[Git] cloning upstream {url}"));
    run_command("git", command, "cloning interbuild upstream source")?;
    Ok(destination.to_path_buf())
}

fn extract_archive_source(
    kind: &str,
    url: &str,
    sha256: Option<&str>,
    work_root: &Path,
    destination: &Path,
    offline: bool,
) -> Result<PathBuf, BuildError> {
    fs::create_dir_all(destination)?;
    let archive_path = work_root.join(format!("{kind}-distfile"));
    copy_location(url, &archive_path, offline)?;
    verify_archive_hash(&archive_path, sha256, url)?;
    unpack_archive(url, &archive_path, destination)?;
    selected_source_root(destination)
}

fn copy_location(source_url: &str, destination: &Path, offline: bool) -> Result<(), BuildError> {
    if let Some(path) = source_url.strip_prefix("file://") {
        fs::copy(path, destination)?;
        return Ok(());
    }

    let local_path = Path::new(source_url);
    if local_path.exists() {
        fs::copy(local_path, destination)?;
        return Ok(());
    }

    if source_url.starts_with("http://") || source_url.starts_with("https://") {
        if offline {
            return Err(BuildError::Unsupported(format!(
                "offline mode cannot fetch interbuild upstream archive `{source_url}`"
            )));
        }
        let response = ureq::get(source_url)
            .call()
            .map_err(|error| BuildError::Fetch(error.to_string()))?;
        let mut reader = response.into_reader();
        let mut file = fs::File::create(destination)?;
        std::io::copy(&mut reader, &mut file)?;
        return Ok(());
    }

    Err(BuildError::Unsupported(format!(
        "unsupported interbuild upstream archive URL `{source_url}`"
    )))
}

fn verify_archive_hash(path: &Path, expected: Option<&str>, url: &str) -> Result<(), BuildError> {
    let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if expected == "SKIP" {
        return Ok(());
    }

    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let actual = format!("{:x}", hasher.finalize());
    if actual == expected {
        return Ok(());
    }

    Err(BuildError::Invalid(format!(
        "interbuild upstream archive sha256 mismatch for `{url}`: expected `{expected}`, got `{actual}`"
    )))
}

fn unpack_archive(url: &str, archive_path: &Path, destination: &Path) -> Result<(), BuildError> {
    match archive_kind(url) {
        Some(ArchiveKind::Tar) => {
            let file = fs::File::open(archive_path)?;
            Archive::new(BufReader::new(file)).unpack(destination)?;
        }
        Some(ArchiveKind::TarGz) => {
            let file = fs::File::open(archive_path)?;
            Archive::new(GzDecoder::new(BufReader::new(file))).unpack(destination)?;
        }
        Some(ArchiveKind::TarXz) => {
            let file = fs::File::open(archive_path)?;
            Archive::new(XzDecoder::new(BufReader::new(file))).unpack(destination)?;
        }
        Some(ArchiveKind::TarZst) => {
            let file = fs::File::open(archive_path)?;
            let decoder = ZstdDecoder::new(BufReader::new(file))?;
            Archive::new(decoder).unpack(destination)?;
        }
        None => {
            return Err(BuildError::Unsupported(format!(
                "unsupported interbuild upstream archive format for `{url}`"
            )));
        }
    }
    Ok(())
}

fn selected_source_root(extract_root: &Path) -> Result<PathBuf, BuildError> {
    if has_build_marker(extract_root) {
        return Ok(extract_root.to_path_buf());
    }

    let child_dirs = fs::read_dir(extract_root)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    match child_dirs.as_slice() {
        [single] => Ok(single.clone()),
        _ => Ok(extract_root.to_path_buf()),
    }
}

fn has_build_marker(source_dir: &Path) -> bool {
    source_dir.join("Cargo.toml").is_file()
        || source_dir.join("CMakeLists.txt").is_file()
        || source_dir.join("go.mod").is_file()
        || source_dir.join("Makefile").is_file()
        || source_dir.join("makefile").is_file()
        || source_dir.join("meson.build").is_file()
        || source_dir.join("build.zig").is_file()
        || source_dir.join("pyproject.toml").is_file()
        || source_dir.join("setup.py").is_file()
        || source_dir.read_dir().ok().is_some_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("nimble")
            })
        })
}

fn is_local_location(location: &str) -> bool {
    location.starts_with("file://") || Path::new(location).exists()
}

fn archive_kind(url: &str) -> Option<ArchiveKind> {
    let path = url.split('?').next().unwrap_or(url);
    if path.ends_with(".tar") {
        Some(ArchiveKind::Tar)
    } else if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
        Some(ArchiveKind::TarGz)
    } else if path.ends_with(".tar.xz") || path.ends_with(".txz") {
        Some(ArchiveKind::TarXz)
    } else if path.ends_with(".tar.zst") || path.ends_with(".tzst") {
        Some(ArchiveKind::TarZst)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveKind {
    Tar,
    TarGz,
    TarXz,
    TarZst,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_kind_ignores_query_string() {
        assert_eq!(
            archive_kind("https://example.invalid/project.tar.gz?download=1"),
            Some(ArchiveKind::TarGz)
        );
    }
}
