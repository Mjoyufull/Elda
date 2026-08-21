use std::fs::File;
use std::io::Read;
use std::path::Path;

use elda_types::ArtifactFormat;

use crate::BuildError;

/// Infer `(name, version)` from the archive root, falling back to the file name.
pub(super) fn infer_identity(
    root: Option<&str>,
    file_name: &str,
) -> (Option<String>, Option<String>) {
    if let Some(root) = root
        && let (Some(name), version) = split_name_version(root)
    {
        return (Some(name), version);
    }
    let stem = strip_archive_suffix(file_name);
    split_name_version(&stem)
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
pub(super) fn split_name_version(stem: &str) -> (Option<String>, Option<String>) {
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

pub(super) fn looks_like_version(segment: &str) -> bool {
    let candidate = segment.strip_prefix('v').unwrap_or(segment);
    let mut chars = candidate.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }
    candidate.contains('.') && candidate.chars().all(|c| c.is_ascii_digit() || c == '.')
}

pub(super) fn infer_architecture(
    path: &Path,
    format: ArtifactFormat,
    root: Option<&str>,
    file_name: &str,
) -> Result<String, BuildError> {
    if matches!(format, ArtifactFormat::Elf | ArtifactFormat::AppImage) {
        let mut file = File::open(path)?;
        let mut header = [0u8; 20];
        file.read_exact(&mut header).map_err(|_| {
            BuildError::Invalid(format!("`{}` has a truncated ELF header", path.display()))
        })?;
        return elf_architecture(&header);
    }

    let hint = format!("{}-{file_name}", root.unwrap_or_default()).to_ascii_lowercase();
    for (needle, architecture) in [
        ("x86_64", "amd64"),
        ("amd64", "amd64"),
        ("aarch64", "arm64"),
        ("arm64", "arm64"),
        ("riscv64", "riscv64"),
        ("ppc64le", "ppc64le"),
        ("powerpc64le", "ppc64le"),
        ("armv7", "armhf"),
        ("armhf", "armhf"),
        ("i686", "i386"),
        ("i386", "i386"),
    ] {
        if hint.contains(needle) {
            return Ok(architecture.to_owned());
        }
    }
    host_architecture().map(ToOwned::to_owned).ok_or_else(|| {
        BuildError::Unsupported(format!(
            "host architecture `{}` has no canonical Elda label",
            std::env::consts::ARCH
        ))
    })
}

pub(super) fn elf_architecture(header: &[u8; 20]) -> Result<String, BuildError> {
    if &header[..4] != b"\x7fELF" || !matches!(header[4], 1 | 2) || !matches!(header[5], 1 | 2) {
        return Err(BuildError::Invalid(
            "artifact has an invalid ELF header".to_owned(),
        ));
    }
    let machine = match header[5] {
        1 => u16::from_le_bytes([header[18], header[19]]),
        2 => u16::from_be_bytes([header[18], header[19]]),
        value => {
            return Err(BuildError::Invalid(format!(
                "ELF has invalid data encoding {value}"
            )));
        }
    };
    let architecture = match (machine, header[4], header[5]) {
        (3, 1, _) => "i386",
        (62, 2, _) => "amd64",
        (40, 1, _) => "armhf",
        (183, 2, _) => "arm64",
        (243, 2, _) => "riscv64",
        (21, 2, 1) => "ppc64le",
        _ => {
            return Err(BuildError::Unsupported(format!(
                "ELF machine {machine}, class {}, data encoding {} is not a supported Elda architecture",
                header[4], header[5]
            )));
        }
    };
    Ok(architecture.to_owned())
}

fn host_architecture() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "x86_64" => Some("amd64"),
        "x86" => Some("i386"),
        "aarch64" => Some("arm64"),
        "arm" => Some("armhf"),
        "riscv64" => Some("riscv64"),
        "powerpc64" if cfg!(target_endian = "little") => Some("ppc64le"),
        _ => None,
    }
}
