use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

use elda_recipe::{ScalarValue, SourceDefinition};
use tar::{Builder, Header};
use tempfile::tempdir;

use super::{ArchiveKind, infer_archive_kind, stage_binary_from_tar};

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

fn source_without_binary() -> SourceDefinition {
    SourceDefinition {
        kind: "url_archive".to_owned(),
        fields: BTreeMap::new(),
        github_release_assets: BTreeMap::new(),
        default_lane: None,
        lanes: BTreeMap::new(),
    }
}

fn source_with_rename(rename: &str) -> SourceDefinition {
    SourceDefinition {
        kind: "url_archive".to_owned(),
        fields: BTreeMap::from([("rename".to_owned(), ScalarValue::String(rename.to_owned()))]),
        github_release_assets: BTreeMap::new(),
        default_lane: None,
        lanes: BTreeMap::new(),
    }
}

#[test]
fn infer_archive_kind_falls_back_to_url_when_cache_file_is_sha256_named() {
    let url =
        "https://github.com/example/p/releases/download/v1/p-1.0-x86_64-unknown-linux-gnu.tar.xz";
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

#[test]
fn infer_archive_kind_handles_url_fragments() {
    let source = source_with_asset("ignored-if-url-matches.tar.gz");
    let path = Path::new("/tmp/abc123def456");
    assert_eq!(
        infer_archive_kind(
            path,
            "https://example.invalid/tool.tar.zst#download",
            &source
        ),
        Some(ArchiveKind::TarZst)
    );
}

#[test]
fn tar_archive_without_binary_uses_single_executable_candidate() {
    let tempdir = tempdir().expect("tempdir should exist");
    let archive_path = tempdir.path().join("payload.tar");
    write_tar(&archive_path, &[TarEntry::executable("tool-1.0/bin/tool")]);
    let bin_dir = tempdir.path().join("stage/usr/bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should exist");

    stage_binary_from_tar(
        &source_without_binary(),
        &archive_path,
        &bin_dir,
        ArchiveKind::Tar,
    )
    .expect("archive should stage");

    assert_eq!(
        fs::read_to_string(bin_dir.join("tool")).expect("staged binary should read"),
        "#!/bin/sh\n"
    );
}

#[test]
fn tar_archive_without_binary_fails_on_multiple_candidates() {
    let tempdir = tempdir().expect("tempdir should exist");
    let archive_path = tempdir.path().join("payload.tar");
    write_tar(
        &archive_path,
        &[
            TarEntry::executable("tool-1.0/bin/tool"),
            TarEntry::executable("tool-1.0/bin/helper"),
        ],
    );
    let bin_dir = tempdir.path().join("stage/usr/bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should exist");

    let error = stage_binary_from_tar(
        &source_without_binary(),
        &archive_path,
        &bin_dir,
        ArchiveKind::Tar,
    )
    .expect_err("ambiguous archive should fail");

    assert!(error.to_string().contains("multiple executable candidates"));
}

#[test]
fn tar_archive_rejects_rename_path_traversal() {
    let tempdir = tempdir().expect("tempdir should exist");
    let archive_path = tempdir.path().join("payload.tar");
    write_tar(&archive_path, &[TarEntry::executable("tool")]);
    let bin_dir = tempdir.path().join("stage/usr/bin");
    fs::create_dir_all(&bin_dir).expect("bin dir should exist");

    let error = stage_binary_from_tar(
        &source_with_rename("../tool"),
        &archive_path,
        &bin_dir,
        ArchiveKind::Tar,
    )
    .expect_err("rename traversal should fail");

    assert!(error.to_string().contains("invalid `rename`"));
    assert!(!tempdir.path().join("stage/usr/tool").exists());
}

struct TarEntry<'a> {
    path: &'a str,
    mode: u32,
}

impl<'a> TarEntry<'a> {
    fn executable(path: &'a str) -> Self {
        Self { path, mode: 0o755 }
    }
}

fn write_tar(path: &Path, entries: &[TarEntry<'_>]) {
    let file = fs::File::create(path).expect("tar file should create");
    let mut builder = Builder::new(file);
    for entry in entries {
        let body = b"#!/bin/sh\n";
        let mut header = Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(entry.mode);
        header.set_cksum();
        builder
            .append_data(&mut header, entry.path, &mut &body[..])
            .expect("tar entry should append");
    }
    builder.finish().expect("tar should finish");
    builder.into_inner().expect("tar should flush").flush().ok();
}
