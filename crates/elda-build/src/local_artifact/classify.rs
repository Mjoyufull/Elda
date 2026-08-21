use elda_types::ArtifactEntryKind;

/// Decide what a member is, so generated metadata can describe its disposition.
pub(super) fn classify(path: &str, executable: bool) -> ArtifactEntryKind {
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
