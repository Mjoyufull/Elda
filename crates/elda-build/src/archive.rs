mod release_url;
mod source;

use std::fs;
use std::io::BufReader;
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::Path;

use flate2::read::GzDecoder;
use liblzma::read::XzDecoder;
use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

use elda_recipe::{RecipeDocument, ScalarValue};
use release_url::resolve_release_asset_url;

use crate::binary_fetch::fetch_binary_source;
use crate::error::BuildError;
use crate::manifest::sha256_file;
use crate::payload_verify::verify_downloaded_payload;
use crate::release_trust;
use crate::{BinaryCache, BinarySourceVerification};

pub fn stage_binary_source(
    recipe: &RecipeDocument,
    cache_src_dir: &Path,
    stage_root: &Path,
    offline: bool,
    configured_caches: &[BinaryCache],
    verification: Option<&BinarySourceVerification>,
    release_trusted_keys: &[String],
) -> Result<(), BuildError> {
    let source = source::materialize_binary_source(
        &recipe.package.source,
        recipe.package.arch.first().map(String::as_str),
    )?;
    let source_url = resolve_source_url(&source)?;
    let expected_sha256 = string_field(&source, "sha256")?;
    let download_path = fetch_binary_source(
        &source_url,
        expected_sha256,
        cache_src_dir,
        configured_caches,
        offline,
    )?;
    let actual_sha256 = sha256_file(&download_path)?;
    if actual_sha256 != expected_sha256 {
        return Err(BuildError::Invalid(format!(
            "downloaded source sha256 mismatch: expected `{expected_sha256}`, got `{actual_sha256}`"
        )));
    }
    if let Some(verification) = verification {
        verify_downloaded_payload(&download_path, verification)?;
    }

    // Release-asset signature trust: if the recipe declares a signature
    // sidecar, fetch and verify it against configured release trust keys.
    let sig_field = string_field_optional(&source, "signature");
    match release_trust::fetch_and_verify_release_sidecar(
        &source_url,
        sig_field,
        &download_path,
        release_trusted_keys,
    )? {
        release_trust::SignatureVerdict::Verified { .. }
        | release_trust::SignatureVerdict::NoSignature => {}
        release_trust::SignatureVerdict::NoTrustKeys => {
            return Err(BuildError::Invalid(
                "release asset declares a signature sidecar but no trusted keys are configured in [trust].release_keys"
                    .to_owned(),
            ));
        }
    }

    if source.kind == "appimage" {
        stage_appimage_payload(recipe, &source, &download_path, &source_url, stage_root)?;
        return Ok(());
    }

    let bin_dir = stage_root.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;

    if let Some(kind) = infer_archive_kind(&download_path, &source_url, &source) {
        stage_binary_from_tar(&source, &download_path, &bin_dir, kind)?;
    } else {
        stage_plain_binary(&source, &download_path, &bin_dir)?;
    }

    Ok(())
}

fn stage_appimage_payload(
    recipe: &RecipeDocument,
    source: &elda_recipe::SourceDefinition,
    downloaded_path: &Path,
    source_url: &str,
    stage_root: &Path,
) -> Result<(), BuildError> {
    let pkg = &recipe.package;
    let binary = string_field(source, "binary")?;
    if binary.contains('/') || binary.contains("..") {
        return Err(BuildError::Invalid(
            "appimage `binary` must be a single filename".to_owned(),
        ));
    }

    let payload_name = appimage_payload_filename(source, source_url)?;
    let version_dir = format!("{}:{}-{}", pkg.epoch, pkg.version, pkg.rel);
    let payload_dir = stage_root
        .join("usr/lib/elda/appimages")
        .join(&pkg.name)
        .join(&version_dir)
        .join("payload");
    fs::create_dir_all(&payload_dir)?;
    let payload_path = payload_dir.join(&payload_name);
    fs::copy(downloaded_path, &payload_path)?;
    fs::set_permissions(&payload_path, fs::Permissions::from_mode(0o755))?;

    let bin_dir = stage_root.join("usr/bin");
    fs::create_dir_all(&bin_dir)?;
    let launcher_path = bin_dir.join(binary);
    if launcher_path.exists() {
        fs::remove_file(&launcher_path)?;
    }

    let relative_target = Path::new("../lib/elda/appimages")
        .join(&pkg.name)
        .join(&version_dir)
        .join("payload")
        .join(&payload_name);

    #[cfg(unix)]
    symlink(&relative_target, &launcher_path)?;

    #[cfg(not(unix))]
    {
        return Err(BuildError::Unsupported(
            "appimage staging requires a unix host".to_owned(),
        ));
    }

    let desktop_integration =
        !matches!(string_field_optional(source, "integration"), Some("none"),);

    if desktop_integration {
        let metadata_mirror = payload_dir
            .parent()
            .ok_or_else(|| BuildError::Invalid("appimage payload path invalid".into()))?
            .join("metadata");
        fs::create_dir_all(&metadata_mirror)?;
        elda_appimage::stage_integration_from_appimage(
            &payload_path,
            stage_root,
            &pkg.name,
            binary,
            None,
            Some(metadata_mirror.as_path()),
        )
        .map_err(|err| {
            BuildError::Invalid(format!(
                "appimage desktop integration failed (SquashFS metadata extraction does not execute the AppImage): {err}"
            ))
        })?;
    }

    Ok(())
}

fn appimage_payload_filename(
    source: &elda_recipe::SourceDefinition,
    source_url: &str,
) -> Result<String, BuildError> {
    if let Some(asset) = string_field_optional(source, "asset") {
        let name = Path::new(asset)
            .file_name()
            .ok_or_else(|| {
                BuildError::Invalid("appimage `asset` must be a plain filename".to_owned())
            })?
            .to_string_lossy()
            .into_owned();
        if name.contains("..") || name.contains('/') {
            return Err(BuildError::Invalid(
                "appimage `asset` must not contain path separators".to_owned(),
            ));
        }
        return Ok(name);
    }

    let segment = source_url
        .rsplit_once('/')
        .map(|(_, tail)| tail)
        .unwrap_or(source_url);
    let base = segment.split('?').next().unwrap_or(segment);
    Path::new(base)
        .file_name()
        .ok_or_else(|| {
            BuildError::Invalid(
                "appimage url source requires `asset = \"...\"` when the URL has no filename"
                    .to_owned(),
            )
        })
        .map(|name| name.to_string_lossy().into_owned())
}

fn resolve_source_url(source: &elda_recipe::SourceDefinition) -> Result<String, BuildError> {
    match source.kind.as_str() {
        "url_archive" => Ok(string_field(source, "url")?.to_owned()),
        "github_release" | "release_asset" => resolve_release_asset_url(source),
        "appimage" => {
            if let Some(url) = string_field_optional(source, "url") {
                Ok(url.to_owned())
            } else {
                resolve_release_asset_url(source)
            }
        }
        other => Err(BuildError::Unsupported(format!(
            "binary source kind `{other}` is not implemented by the current build slice"
        ))),
    }
}

fn stage_plain_binary(
    source: &elda_recipe::SourceDefinition,
    downloaded_path: &Path,
    bin_dir: &Path,
) -> Result<(), BuildError> {
    let install_name = string_field_optional(source, "rename")
        .or_else(|| string_field_optional(source, "binary"))
        .map(ToOwned::to_owned)
        .or_else(|| {
            downloaded_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .ok_or_else(|| BuildError::Invalid("could not derive a binary install name".to_owned()))?;
    let destination = bin_dir.join(install_name);
    fs::copy(downloaded_path, &destination)?;
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o755))?;

    Ok(())
}

fn stage_binary_from_tar(
    source: &elda_recipe::SourceDefinition,
    downloaded_path: &Path,
    bin_dir: &Path,
    kind: ArchiveKind,
) -> Result<(), BuildError> {
    let requested_binary = string_field(source, "binary")?;
    let install_name = string_field_optional(source, "rename")
        .map(ToOwned::to_owned)
        .or_else(|| {
            Path::new(requested_binary)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .ok_or_else(|| {
            BuildError::Invalid(format!(
                "binary source `{}` requires a valid `binary` path",
                source.kind
            ))
        })?;
    let destination = bin_dir.join(install_name);
    let requested_path = Path::new(requested_binary);
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

/// Classify tarball compression from a filename or URL last segment.
///
/// Payloads in the content-addressed cache are stored as `<sha256>` with no
/// extension, so callers must fall back to the download URL or recipe `asset`.
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

fn infer_archive_kind(
    downloaded_path: &Path,
    source_url: &str,
    source: &elda_recipe::SourceDefinition,
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

fn string_field<'a>(
    source: &'a elda_recipe::SourceDefinition,
    key: &str,
) -> Result<&'a str, BuildError> {
    string_field_optional(source, key).ok_or_else(|| {
        BuildError::Invalid(format!("source.kind `{}` is missing `{key}`", source.kind))
    })
}

fn string_field_optional<'a>(
    source: &'a elda_recipe::SourceDefinition,
    key: &str,
) -> Option<&'a str> {
    match source.fields.get(key) {
        Some(ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveKind {
    Tar,
    TarGz,
    TarZst,
    TarXz,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use elda_recipe::{ScalarValue, SourceDefinition};

    use super::{ArchiveKind, infer_archive_kind};

    fn source_with_asset(asset: &str) -> SourceDefinition {
        SourceDefinition {
            kind: "github_release".to_owned(),
            fields: BTreeMap::from([
                ("asset".to_owned(), ScalarValue::String(asset.to_owned())),
                ("sha256".to_owned(), ScalarValue::String("x".to_owned())),
            ]),
            github_release_assets: BTreeMap::new(),
            default_lane: None,
            lanes: BTreeMap::new(),
        }
    }

    #[test]
    fn infer_archive_kind_falls_back_to_url_when_cache_file_is_sha256_named() {
        let url = "https://github.com/example/p/releases/download/v1/p-1.0-x86_64-unknown-linux-gnu.tar.xz";
        let source = source_with_asset("ignored-if-url-matches.tar.gz");
        let path = Path::new(
            "/var/cache/elda/src/62ede54ea3e30ae00b378bf7337f0e6ec1cbbb32f328d06cbd9084622e31e2d4",
        );
        assert_eq!(
            infer_archive_kind(path, url, &source),
            Some(ArchiveKind::TarXz)
        );
    }

    #[test]
    fn infer_archive_kind_uses_asset_when_url_has_no_suffix() {
        let source = source_with_asset("bundle.tar.gz");
        let path = Path::new("/tmp/abc123def456");
        assert_eq!(
            infer_archive_kind(path, "https://example.invalid/dl/abc", &source),
            Some(ArchiveKind::TarGz)
        );
    }

    #[cfg(unix)]
    #[test]
    fn appimage_url_lane_stages_payload_symlink_and_desktop_integration() {
        use elda_recipe::parse_pkg_lua;
        use std::fs;
        use std::path::PathBuf;
        use tempfile::tempdir;

        use crate::manifest::sha256_file;

        let demo_fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../ref/gearlever/src/assets/demo.AppImage");
        if !demo_fixture.is_file() {
            return;
        }

        let tmp = tempdir().expect("tempdir");
        let fixture = tmp.path().join("demo.AppImage");
        fs::copy(&demo_fixture, &fixture).expect("copy demo AppImage");
        let digest = sha256_file(&fixture).expect("sha256");

        let cache = tmp.path().join("cache-src");
        fs::create_dir_all(&cache).expect("cache dir should be created");
        fs::copy(&fixture, cache.join(&digest)).expect("fixture should copy into cache");

        let stage_parent = tempdir().expect("stage parent");
        let stage_root = stage_parent.path().join("stage");
        fs::create_dir_all(&stage_root).expect("stage root should be created");

        let url = format!("file://{}", fixture.display());
        let pkg_lua = format!(
            r#"pkg = {{
  name = "demo",
  epoch = 1,
  version = "2",
  rel = 3,
  arch = {{ "amd64" }},
  kind = "normal",
  source = {{
    kind = "appimage",
    url = "{url}",
    sha256 = "{digest}",
    binary = "demo",
    asset = "demo.AppImage",
  }},
  depends = {{}},
  makedepends = {{}},
  checkdepends = {{}},
  recommends = {{}},
  suggests = {{}},
  supplements = {{}},
  enhances = {{}},
  provides = {{}},
  conflicts = {{}},
  replaces = {{}},
  conffiles = {{}},
}}
"#,
            url = url,
            digest = digest,
        );

        let recipe = parse_pkg_lua(Path::new("pkg.lua"), &pkg_lua).expect("parse");
        super::stage_binary_source(&recipe, &cache, &stage_root, true, &[], None, &[])
            .expect("stage");

        let payload = stage_root.join("usr/lib/elda/appimages/demo/1:2-3/payload/demo.AppImage");
        assert!(payload.is_file(), "expected {}", payload.display());
        let launcher = stage_root.join("usr/bin/demo");
        assert!(
            fs::symlink_metadata(&launcher)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            "launcher should be a symlink"
        );

        let desktop_path = stage_root.join("usr/share/applications/demo.desktop");
        assert!(
            desktop_path.is_file(),
            "expected {}",
            desktop_path.display()
        );
        let desktop_txt = fs::read_to_string(&desktop_path).expect("desktop");
        assert!(
            desktop_txt.contains("Exec=/usr/bin/demo"),
            "desktop should launch Elda symlink: {desktop_txt}"
        );

        let mirror =
            stage_root.join("usr/lib/elda/appimages/demo/1:2-3/metadata/helloworld.desktop");
        assert!(
            mirror.is_file(),
            "expected upstream desktop mirror at {}",
            mirror.display()
        );
    }

    #[cfg(unix)]
    #[test]
    fn appimage_integration_none_skips_desktop_files() {
        use elda_recipe::parse_pkg_lua;
        use std::fs;
        use std::path::PathBuf;
        use tempfile::tempdir;

        use crate::manifest::sha256_file;

        let demo_fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../ref/gearlever/src/assets/demo.AppImage");
        if !demo_fixture.is_file() {
            return;
        }

        let tmp = tempdir().expect("tempdir");
        let fixture = tmp.path().join("demo.AppImage");
        fs::copy(&demo_fixture, &fixture).expect("copy demo AppImage");
        let digest = sha256_file(&fixture).expect("sha256");

        let cache = tmp.path().join("cache-src");
        fs::create_dir_all(&cache).expect("cache dir should be created");
        fs::copy(&fixture, cache.join(&digest)).expect("fixture should copy into cache");

        let stage_parent = tempdir().expect("stage parent");
        let stage_root = stage_parent.path().join("stage");
        fs::create_dir_all(&stage_root).expect("stage root should be created");

        let url = format!("file://{}", fixture.display());
        let pkg_lua = format!(
            r#"pkg = {{
  name = "demo",
  epoch = 1,
  version = "2",
  rel = 3,
  arch = {{ "amd64" }},
  kind = "normal",
  source = {{
    kind = "appimage",
    url = "{url}",
    sha256 = "{digest}",
    binary = "demo",
    asset = "demo.AppImage",
    integration = "none",
  }},
  depends = {{}},
  makedepends = {{}},
  checkdepends = {{}},
  recommends = {{}},
  suggests = {{}},
  supplements = {{}},
  enhances = {{}},
  provides = {{}},
  conflicts = {{}},
  replaces = {{}},
  conffiles = {{}},
}}
"#,
            url = url,
            digest = digest,
        );

        let recipe = parse_pkg_lua(Path::new("pkg.lua"), &pkg_lua).expect("parse");
        super::stage_binary_source(&recipe, &cache, &stage_root, true, &[], None, &[])
            .expect("stage");

        assert!(
            !stage_root
                .join("usr/share/applications/demo.desktop")
                .exists(),
            "desktop integration disabled — no stub desktop expected"
        );
    }
}
