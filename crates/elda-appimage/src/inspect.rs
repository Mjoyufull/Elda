use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use backhand::{FilesystemReader, InnerNode};
use serde::Serialize;

use crate::desktop::{desktop_entry_section, parse_desktop};
use crate::error::AppImageError;
use crate::offset::{appimage_type_magic, squashfs_payload_offset};

const FUSE_HINT: &str = "Type 2 AppImages normally mount their SquashFS payload via FUSE; hosts without FUSE may need `--appimage-extract-and-run` upstream support (not enabled by Elda by default).";

#[derive(Debug, Clone, Serialize)]
pub struct InspectReport {
    pub path: String,
    pub generation: u8,
    pub squashfs_offset: u64,
    pub desktop_candidates: Vec<String>,
    pub primary_desktop_path: Option<String>,
    pub desktop_name: Option<String>,
    pub desktop_exec_original: Option<String>,
    pub desktop_icon_raw: Option<String>,
    pub apprun_path: Option<String>,
    pub icon_candidates: Vec<String>,
    pub metainfo_candidates: Vec<String>,
    pub fuse_note: &'static str,
}

pub fn inspect_appimage(path: &Path) -> Result<InspectReport, AppImageError> {
    let bytes = std::fs::read(path).map_err(|e| AppImageError::io(path, e))?;

    let generation = appimage_type_magic(&bytes).ok_or(AppImageError::UnsupportedGeneration)?;
    let offset = squashfs_payload_offset(&bytes)?;

    let squash_reader = BufReader::new(File::open(path).map_err(|e| AppImageError::io(path, e))?);
    let fs = FilesystemReader::from_reader_with_offset(squash_reader, offset)
        .map_err(|e| AppImageError::Squashfs(e.to_string()))?;

    let mut desktop_candidates = Vec::new();
    let mut icon_candidates = Vec::new();
    let mut metainfo_candidates = Vec::new();
    let mut apprun_path = None::<String>;

    for node in fs.files() {
        let p = node.fullpath.to_string_lossy().into_owned();
        let pl = p.to_ascii_lowercase();

        if pl.ends_with("/apprun") || p == "/AppRun" {
            apprun_path = Some(p.clone());
        }

        if pl.ends_with(".desktop") && !pl.contains("mimeinfo.cache") {
            desktop_candidates.push(p);
            continue;
        }

        if pl.contains("/metainfo/")
            && (pl.ends_with(".metainfo.xml") || pl.ends_with(".appdata.xml"))
        {
            metainfo_candidates.push(p);
            continue;
        }

        if pl.contains("/icons/") && !pl.ends_with('/') {
            let looks_icon = pl.ends_with(".png")
                || pl.ends_with(".svg")
                || pl.ends_with(".xpm")
                || pl.ends_with(".jpg")
                || pl.ends_with(".jpeg");
            if looks_icon {
                icon_candidates.push(p);
            }
        }
    }

    desktop_candidates.sort();

    let primary = select_primary_desktop(&desktop_candidates);

    let mut desktop_name = None;
    let mut desktop_exec_original = None;
    let mut desktop_icon_raw = None;

    if let Some(rel) = &primary
        && let Ok(raw) = read_squash_text_file(&fs, rel)
        && let Ok(parsed) = parse_desktop(&raw)
        && let Some(map) = desktop_entry_section(&parsed)
    {
        desktop_name.clone_from(&map.get("Name").cloned());
        desktop_exec_original.clone_from(&map.get("Exec").cloned());
        desktop_icon_raw.clone_from(&map.get("Icon").cloned());
    }

    Ok(InspectReport {
        path: path.display().to_string(),
        generation,
        squashfs_offset: offset,
        desktop_candidates,
        primary_desktop_path: primary,
        desktop_name,
        desktop_exec_original,
        desktop_icon_raw,
        apprun_path,
        icon_candidates,
        metainfo_candidates,
        fuse_note: FUSE_HINT,
    })
}

fn select_primary_desktop(paths: &[String]) -> Option<String> {
    let mut best: Option<(i32, String)> = None;

    for path in paths {
        let score = desktop_candidate_score(path);
        let replace = match &best {
            None => true,
            Some((best_score, best_path)) => {
                score > *best_score || (score == *best_score && path < best_path)
            }
        };
        if replace {
            best = Some((score, path.clone()));
        }
    }

    best.map(|(_, path)| path)
}

pub(crate) fn pick_primary_desktop_path(paths: &[String]) -> Option<String> {
    select_primary_desktop(paths)
}

fn desktop_candidate_score(path: &str) -> i32 {
    let pl = path.to_ascii_lowercase();
    let mut score = 0;
    if pl.contains("/usr/share/applications/") {
        score += 40;
    }
    if pl.starts_with("/usr/") {
        score += 10;
    }
    if !pl.contains("vendor") && !pl.contains("screensaver") {
        score += 5;
    }
    if path.rsplit_once('/').is_some_and(|(_, name)| {
        let nl = name.to_ascii_lowercase();
        nl.contains("default") || nl.contains("main")
    }) {
        score += 8;
    }
    score
}

fn read_squash_text_file(
    fs: &FilesystemReader<'_>,
    fullpath: &str,
) -> Result<String, AppImageError> {
    let node = fs
        .files()
        .find(|node| node.fullpath == Path::new(fullpath))
        .ok_or_else(|| AppImageError::Squashfs("desktop node missing".to_owned()))?;

    let InnerNode::File(file_obj) = &node.inner else {
        return Err(AppImageError::Squashfs(
            "desktop node is not a file".to_owned(),
        ));
    };

    let mut reader = fs.file(file_obj).reader();
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut buf)
        .map_err(|e| AppImageError::Squashfs(e.to_string()))?;

    String::from_utf8(buf).map_err(|e| AppImageError::Squashfs(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn inspect_gearlever_demo_fixture() {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("../../ref/gearlever/src/assets/demo.AppImage");
        if !p.exists() {
            return;
        }

        let report = inspect_appimage(&p).expect("inspect demo AppImage");
        assert_eq!(report.generation, 2);
        assert_eq!(report.squashfs_offset, 189632);
        assert!(
            report
                .desktop_candidates
                .iter()
                .any(|entry| entry.ends_with("helloworld.desktop")),
            "{:?}",
            report.desktop_candidates
        );
        assert_eq!(
            report.primary_desktop_path.as_deref(),
            Some("/helloworld.desktop")
        );
        assert_eq!(report.desktop_name.as_deref(), Some("helloworld-appimage"));
        assert!(report.apprun_path.as_deref() == Some("/AppRun"));
    }
}
