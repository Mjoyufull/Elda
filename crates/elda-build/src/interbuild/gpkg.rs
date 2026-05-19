use std::path::Path;

use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

use crate::error::BuildError;

/// Result of a successful GPKG fast-path extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpkgResult {
    pub used_binary: bool,
    pub gpkg_use: Vec<String>,
}

/// Attempt the GPKG binary fast-path for a Gentoo package.
///
/// If a binhost URL is configured and the binary package is available
/// with compatible USE flags, extracts the payload directly into the
/// stage root. Returns `Ok(None)` when the fast-path is not available
/// and the caller should fall through to a normal source build.
///
/// GPKG format (GLEP-78):
///   outer `.gpkg.tar` containing a `gpkg-1/` directory with:
///   - `Manifest`        — checksums
///   - `metadata.tar.zst` — package metadata key-value pairs
///   - `image.tar.zst`    — the filesystem payload
pub fn try_gpkg_fast_path(
    binhost_url: Option<&str>,
    category: &str,
    package_atom: &str,
    required_use: &[String],
    stage_root: &Path,
) -> Result<Option<GpkgResult>, BuildError> {
    let Some(binhost) = binhost_url else {
        return Ok(None);
    };

    let gpkg_url = format!(
        "{}/{}/{}.gpkg.tar",
        binhost.trim_end_matches('/'),
        category,
        package_atom
    );

    // Fetch the outer GPKG tar
    let response = match ureq::get(&gpkg_url).call() {
        Ok(resp) => resp,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(ureq::Error::Status(code, _)) => {
            return Err(BuildError::Fetch(format!(
                "gpkg binhost returned {code} for {gpkg_url}"
            )));
        }
        Err(error) => {
            return Err(BuildError::Fetch(format!(
                "gpkg binhost unreachable: {error}"
            )));
        }
    };

    let mut outer_bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut outer_bytes)
        .map_err(|error| BuildError::Fetch(format!("gpkg download failed: {error}")))?;

    // Parse the outer tar
    let mut outer = Archive::new(std::io::Cursor::new(&outer_bytes));
    let mut metadata_bytes: Option<Vec<u8>> = None;
    let mut image_bytes: Option<Vec<u8>> = None;

    for entry in outer
        .entries()
        .map_err(|error| BuildError::Invalid(format!("gpkg outer tar is invalid: {error}")))?
    {
        let mut entry = entry
            .map_err(|error| BuildError::Invalid(format!("gpkg outer tar entry error: {error}")))?;
        let path = entry
            .path()
            .map_err(|error| BuildError::Invalid(format!("gpkg entry path error: {error}")))?;
        let path_str = path.to_string_lossy();

        if path_str.ends_with("/metadata.tar.zst") || path_str == "metadata.tar.zst" {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf).map_err(BuildError::Io)?;
            metadata_bytes = Some(buf);
        } else if path_str.ends_with("/image.tar.zst") || path_str == "image.tar.zst" {
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf).map_err(BuildError::Io)?;
            image_bytes = Some(buf);
        }
    }

    let metadata_bytes = metadata_bytes.ok_or_else(|| {
        BuildError::Invalid("gpkg archive is missing metadata.tar.zst".to_owned())
    })?;
    let image_bytes = image_bytes
        .ok_or_else(|| BuildError::Invalid("gpkg archive is missing image.tar.zst".to_owned()))?;

    // Parse metadata to extract USE flags
    let gpkg_use = extract_gpkg_use_flags(&metadata_bytes)?;

    // Check USE flag compatibility
    if !check_use_compatibility(required_use, &gpkg_use) {
        return Ok(None);
    }

    // Extract image.tar.zst into stage root
    extract_gpkg_image(&image_bytes, stage_root)?;

    Ok(Some(GpkgResult {
        used_binary: true,
        gpkg_use,
    }))
}

/// Extract USE flags from GPKG metadata.
///
/// The metadata tarball contains key-value text files. The `USE` file
/// lists the USE flags the binary was built with.
fn extract_gpkg_use_flags(metadata_bytes: &[u8]) -> Result<Vec<String>, BuildError> {
    let decoder = ZstdDecoder::new(std::io::Cursor::new(metadata_bytes)).map_err(|error| {
        BuildError::Invalid(format!("gpkg metadata decompression failed: {error}"))
    })?;
    let mut archive = Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|error| BuildError::Invalid(format!("gpkg metadata tar is invalid: {error}")))?
    {
        let mut entry = entry
            .map_err(|error| BuildError::Invalid(format!("gpkg metadata entry error: {error}")))?;
        let path = entry
            .path()
            .map_err(|error| BuildError::Invalid(format!("gpkg metadata path error: {error}")))?;

        if path.file_name().and_then(|n| n.to_str()) == Some("USE") {
            let mut use_text = String::new();
            std::io::Read::read_to_string(&mut entry, &mut use_text).map_err(BuildError::Io)?;
            return Ok(use_text.split_whitespace().map(|s| s.to_owned()).collect());
        }
    }

    Ok(Vec::new())
}

/// Check whether the GPKG's USE flags are compatible with
/// the required USE set.
///
/// Every flag in `required` must be present in `available`.
/// Flags prefixed with `-` in available indicate disabled flags;
/// required flags prefixed with `-` must match a disabled flag.
fn check_use_compatibility(required: &[String], available: &[String]) -> bool {
    for req in required {
        let negated = req.starts_with('-');
        let flag = req.trim_start_matches('-');
        if negated {
            // Require the flag to be absent or explicitly negated
            if available.contains(&flag.to_owned()) {
                return false;
            }
        } else {
            // Require the flag to be present
            if !available.contains(&flag.to_owned()) {
                return false;
            }
        }
    }
    true
}

/// Extract the GPKG image payload into the stage root.
fn extract_gpkg_image(image_bytes: &[u8], stage_root: &Path) -> Result<(), BuildError> {
    let decoder = ZstdDecoder::new(std::io::Cursor::new(image_bytes)).map_err(|error| {
        BuildError::Invalid(format!("gpkg image decompression failed: {error}"))
    })?;
    let mut archive = Archive::new(decoder);
    archive
        .unpack(stage_root)
        .map_err(|error| BuildError::Invalid(format!("gpkg image extraction failed: {error}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_compatibility_all_present() {
        let required = vec!["X".to_owned(), "gtk".to_owned()];
        let available = vec!["X".to_owned(), "gtk".to_owned(), "qt5".to_owned()];
        assert!(check_use_compatibility(&required, &available));
    }

    #[test]
    fn use_compatibility_missing_flag() {
        let required = vec!["X".to_owned(), "wayland".to_owned()];
        let available = vec!["X".to_owned(), "gtk".to_owned()];
        assert!(!check_use_compatibility(&required, &available));
    }

    #[test]
    fn use_compatibility_negated_flag() {
        let required = vec!["-systemd".to_owned()];
        let available = vec!["X".to_owned(), "gtk".to_owned()];
        assert!(check_use_compatibility(&required, &available));
    }

    #[test]
    fn use_compatibility_negated_but_present_fails() {
        let required = vec!["-systemd".to_owned()];
        let available = vec!["X".to_owned(), "systemd".to_owned()];
        assert!(!check_use_compatibility(&required, &available));
    }

    #[test]
    fn empty_required_always_compatible() {
        let available = vec!["X".to_owned(), "gtk".to_owned()];
        assert!(check_use_compatibility(&[], &available));
    }

    #[test]
    fn no_binhost_returns_none() {
        let result = try_gpkg_fast_path(
            None,
            "dev-util",
            "ripgrep-14.1.0",
            &[],
            Path::new("/tmp/stage"),
        )
        .expect("empty binhost should be accepted");
        assert!(result.is_none());
    }
}
