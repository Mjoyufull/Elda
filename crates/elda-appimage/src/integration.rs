use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use backhand::{FilesystemReader, InnerNode};

use crate::desktop::{desktop_entry_section, parse_desktop, rewrite_desktop_exec_sections};
use crate::error::AppImageError;
use crate::inspect::pick_primary_desktop_path;
use crate::offset::squashfs_payload_offset;

#[derive(Debug, Clone, Default)]
pub struct IntegrationOutcome {
    pub desktop_installed: Option<PathBuf>,
    pub metainfo_installed: Vec<PathBuf>,
    pub icons_installed: Vec<PathBuf>,
    pub metadata_mirror_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

/// Extract `.desktop`, icons, and AppStream metadata from a Type‑2 AppImage without executing it.
pub fn stage_integration_from_appimage(
    appimage_path: &Path,
    stage_root: &Path,
    pkg_name: &str,
    launcher_binary: &str,
    primary_desktop_rel: Option<&str>,
    metadata_mirror_root: Option<&Path>,
) -> Result<IntegrationOutcome, AppImageError> {
    let mut outcome = IntegrationOutcome::default();

    let bytes = std::fs::read(appimage_path).map_err(|e| AppImageError::io(appimage_path, e))?;
    let offset = squashfs_payload_offset(&bytes)?;

    let squash_reader =
        BufReader::new(File::open(appimage_path).map_err(|e| AppImageError::io(appimage_path, e))?);
    let fs = FilesystemReader::from_reader_with_offset(squash_reader, offset)
        .map_err(|e| AppImageError::Squashfs(e.to_string()))?;

    let mut desktop_paths = Vec::new();
    for node in fs.files() {
        let p = node.fullpath.to_string_lossy().into_owned();
        let pl = p.to_ascii_lowercase();
        if pl.ends_with(".desktop") && !pl.contains("mimeinfo.cache") {
            desktop_paths.push(p);
        }
    }
    desktop_paths.sort();

    let primary = primary_desktop_rel
        .map(str::to_owned)
        .or_else(|| pick_primary_desktop_path(&desktop_paths));

    let Some(primary_path) = primary else {
        outcome.warnings.push(
            "no `.desktop` entry inside SquashFS; wrote minimal launcher desktop stub".into(),
        );
        write_fallback_desktop(stage_root, pkg_name, launcher_binary)?;
        outcome.desktop_installed = Some(PathBuf::from(format!(
            "usr/share/applications/{pkg_name}.desktop"
        )));
        return Ok(outcome);
    };

    let raw_txt = read_squash_utf8(&fs, &primary_path)?;

    let desktop_body = match rewrite_desktop_exec_sections(&raw_txt, launcher_binary) {
        Ok(body) => body,
        Err(err) => {
            outcome.warnings.push(format!(
                "could not rewrite upstream desktop Exec ({err}); using minimal stub"
            ));
            fallback_desktop_body(pkg_name, launcher_binary)
        }
    };

    let desktop_rel = PathBuf::from(format!("usr/share/applications/{pkg_name}.desktop"));
    let desktop_abs = stage_root.join(&desktop_rel);
    create_parent_dir(&desktop_abs)?;
    fs::write(&desktop_abs, desktop_body.as_bytes())
        .map_err(|e| AppImageError::io(&desktop_abs, e))?;
    outcome.desktop_installed = Some(desktop_rel);

    if let Some(mirror_root) = metadata_mirror_root {
        fs::create_dir_all(mirror_root).map_err(|e| AppImageError::io(mirror_root, e))?;
        let fname = Path::new(&primary_path)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("upstream.desktop"));
        let mirror_desktop = mirror_root.join(fname);
        fs::write(&mirror_desktop, raw_txt.as_bytes())
            .map_err(|e| AppImageError::io(&mirror_desktop, e))?;
        outcome
            .metadata_mirror_paths
            .push(rel_path_under_stage(&mirror_desktop, stage_root));
    }

    if let Ok(parsed) = parse_desktop(&raw_txt)
        && let Some(map) = desktop_entry_section(&parsed)
        && let Some(icon) = map.get("Icon")
    {
        match resolve_icon_and_install(&fs, stage_root, pkg_name, icon.as_str()) {
            Ok(paths) => {
                outcome.icons_installed.extend(paths.clone());
                if let Some(mirror_root) = metadata_mirror_root {
                    mirror_icon_copies(stage_root, mirror_root, &paths, &mut outcome);
                }
            }
            Err(err) => outcome
                .warnings
                .push(format!("icon extraction skipped: {err}")),
        }
    }

    copy_metainfo_files(
        &fs,
        stage_root,
        pkg_name,
        metadata_mirror_root,
        &mut outcome,
    )?;

    Ok(outcome)
}

fn mirror_icon_copies(
    stage_root: &Path,
    mirror_root: &Path,
    icon_rels: &[PathBuf],
    outcome: &mut IntegrationOutcome,
) {
    for rel in icon_rels {
        let staged = stage_root.join(rel);
        if !staged.is_file() {
            continue;
        }
        let Some(fname) = staged.file_name() else {
            continue;
        };
        let mirror_file = mirror_root.join(fname);
        if fs::copy(&staged, &mirror_file).is_ok() {
            outcome
                .metadata_mirror_paths
                .push(rel_path_under_stage(&mirror_file, stage_root));
        }
    }
}

fn rel_path_under_stage(full: &Path, stage_root: &Path) -> PathBuf {
    full.strip_prefix(stage_root).unwrap_or(full).to_path_buf()
}

fn create_parent_dir(path: &Path) -> Result<(), AppImageError> {
    let parent = path.parent().ok_or_else(|| {
        AppImageError::Squashfs(format!("`{}` has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|e| AppImageError::io(parent, e))
}

fn read_squash_utf8(fs: &FilesystemReader<'_>, fullpath: &str) -> Result<String, AppImageError> {
    let bytes = read_squash_file_bytes(fs, fullpath)?;
    String::from_utf8(bytes).map_err(|e| AppImageError::Squashfs(e.to_string()))
}

fn read_squash_file_bytes(
    fs: &FilesystemReader<'_>,
    fullpath: &str,
) -> Result<Vec<u8>, AppImageError> {
    let path = Path::new(fullpath);
    let node = fs
        .files()
        .find(|node| node.fullpath == path)
        .ok_or_else(|| AppImageError::Squashfs(format!("missing `{fullpath}` in SquashFS")))?;

    let InnerNode::File(file_obj) = &node.inner else {
        return Err(AppImageError::Squashfs(format!(
            "`{fullpath}` is not a file"
        )));
    };

    let mut reader = fs.file(file_obj).reader();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut buf)
        .map_err(|e| AppImageError::Squashfs(e.to_string()))?;
    Ok(buf)
}

fn resolve_icon_and_install(
    fs: &FilesystemReader<'_>,
    stage_root: &Path,
    _pkg_name: &str,
    icon: &str,
) -> Result<Vec<PathBuf>, AppImageError> {
    let icon = icon.trim();
    if icon.is_empty() {
        return Err(AppImageError::Squashfs("empty Icon=".into()));
    }

    let mut installed = Vec::new();

    if icon.contains('/') {
        let squash_path = Path::new(icon);
        let squash_key = squash_path.to_string_lossy().to_string();
        let bytes = read_squash_file_bytes(fs, &squash_key)?;
        let rel = strip_leading_slash(squash_path);
        let dest = stage_root.join(rel);
        create_parent_dir(&dest)?;
        fs::write(&dest, &bytes).map_err(|e| AppImageError::io(&dest, e))?;
        installed.push(rel_path_under_stage(&dest, stage_root));
        return Ok(installed);
    }

    let mut hits = Vec::new();
    for node in fs.files() {
        let InnerNode::File(_) = &node.inner else {
            continue;
        };
        let p = node.fullpath.to_string_lossy().into_owned();
        let pl = p.to_ascii_lowercase();
        if !pl.contains("/icons/hicolor/") || !pl.contains("/apps/") {
            continue;
        }
        let Some(stem) = node.fullpath.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem.eq_ignore_ascii_case(icon) {
            hits.push(p);
        }
    }

    hits.sort();

    let Some(best) = hits.first().cloned() else {
        return Err(AppImageError::Squashfs(format!(
            "could not resolve theme icon `{icon}` inside SquashFS"
        )));
    };

    let bytes = read_squash_file_bytes(fs, &best)?;
    let rel_inside = strip_leading_slash(Path::new(&best));
    let dest = stage_root.join(rel_inside);
    create_parent_dir(&dest)?;
    fs::write(&dest, &bytes).map_err(|e| AppImageError::io(&dest, e))?;
    installed.push(rel_path_under_stage(&dest, stage_root));

    Ok(installed)
}

fn strip_leading_slash(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    PathBuf::from(s.trim_start_matches('/'))
}

fn copy_metainfo_files(
    fs: &FilesystemReader<'_>,
    stage_root: &Path,
    _pkg_name: &str,
    metadata_mirror_root: Option<&Path>,
    outcome: &mut IntegrationOutcome,
) -> Result<(), AppImageError> {
    let metainfo_dir = stage_root.join("usr/share/metainfo");
    fs::create_dir_all(&metainfo_dir).map_err(|e| AppImageError::io(&metainfo_dir, e))?;

    let mut seen = BTreeSet::new();

    for node in fs.files() {
        let InnerNode::File(_) = &node.inner else {
            continue;
        };
        let p = node.fullpath.to_string_lossy().into_owned();
        let pl = p.to_ascii_lowercase();
        if !pl.contains("/metainfo/") {
            continue;
        }
        if !(pl.ends_with(".metainfo.xml") || pl.ends_with(".appdata.xml")) {
            continue;
        }

        let bytes = read_squash_file_bytes(fs, &p)?;
        let fname = Path::new(&p)
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("unknown.xml"));
        let fname_key = fname.to_string_lossy().to_string();
        if !seen.insert(fname_key.clone()) {
            continue;
        }

        let dest = metainfo_dir.join(fname);
        fs::write(&dest, &bytes).map_err(|e| AppImageError::io(&dest, e))?;
        outcome
            .metainfo_installed
            .push(rel_path_under_stage(&dest, stage_root));

        if let Some(mirror_root) = metadata_mirror_root
            && let Some(file_name) = dest.file_name()
        {
            let mirror_file = mirror_root.join(file_name);
            let _ = fs::copy(&dest, &mirror_file);
            outcome
                .metadata_mirror_paths
                .push(rel_path_under_stage(&mirror_file, stage_root));
        }
    }

    Ok(())
}

fn write_fallback_desktop(
    stage_root: &Path,
    pkg_name: &str,
    launcher_binary: &str,
) -> Result<(), AppImageError> {
    let body = fallback_desktop_body(pkg_name, launcher_binary);
    let desktop_abs = stage_root.join(format!("usr/share/applications/{pkg_name}.desktop"));
    create_parent_dir(&desktop_abs)?;
    fs::write(&desktop_abs, body.as_bytes()).map_err(|e| AppImageError::io(&desktop_abs, e))?;
    Ok(())
}

fn fallback_desktop_body(pkg_name: &str, launcher_binary: &str) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName={pkg_name}\nExec=/usr/bin/{launcher_binary}\nTerminal=false\n"
    )
}
