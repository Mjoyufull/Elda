use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use elda_types::{ArtifactEntryKind, ArtifactFormat};
use tempfile::TempDir;

use super::{
    MAX_ARCHIVE_MEMBER_BYTES, MAX_ARCHIVE_UNPACKED_BYTES, checked_archive_path, classify,
    elf_architecture, identify, infer_identity, looks_like_version, push_entry, split_name_version,
    survey,
};

fn release_tarball(dir: &std::path::Path, root: &str) -> std::path::PathBuf {
    let staging = dir.join("stage").join(root);
    fs::create_dir_all(staging.join("completions")).expect("staging dirs");

    let binary = staging.join("delta");
    fs::write(&binary, b"#!/bin/sh\n").expect("binary");
    let mut perms = fs::metadata(&binary).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&binary, perms).expect("chmod");

    fs::write(staging.join("delta.1"), b".TH DELTA 1\n").expect("man");
    fs::write(staging.join("LICENSE"), b"MIT\n").expect("license");
    fs::write(staging.join("README.md"), b"# delta\n").expect("readme");
    fs::write(staging.join("completions/delta.bash"), b"complete\n").expect("completion");
    fs::write(staging.join("delta.exe"), b"MZ").expect("windows artifact");

    let archive_path = dir.join("delta-linux-x86_64.tar.gz");
    let file = fs::File::create(&archive_path).expect("archive");
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    builder
        .append_dir_all(root, &staging)
        .expect("append staged tree");
    builder.into_inner().expect("tar").finish().expect("gz");
    archive_path
}

#[test]
fn identify_reads_magic_not_extension() {
    let dir = TempDir::new().expect("tempdir");
    let archive = release_tarball(dir.path(), "delta-0.18.2-x86_64-unknown-linux-gnu");
    assert_eq!(identify(&archive), Some(ArtifactFormat::TarGz));

    // A gzip file wearing the wrong extension is still gzip.
    let lying = dir.path().join("actually-gzip.zip");
    fs::copy(&archive, &lying).expect("copy");
    assert_eq!(identify(&lying), Some(ArtifactFormat::TarGz));

    let elf = dir.path().join("some-binary");
    let mut handle = fs::File::create(&elf).expect("elf");
    handle.write_all(b"\x7fELF\x02\x01\x01\x00").expect("write");
    assert_eq!(identify(&elf), Some(ArtifactFormat::Elf));

    // An AppImage is also an ELF; the type magic at bytes 8..11 is what
    // separates them, and misfiling one as a plain binary loses its payload.
    let appimage = dir.path().join("Some-x86_64.AppImage");
    let mut bytes = vec![0u8; 128];
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[8..11].copy_from_slice(&[0x41, 0x49, 0x02]);
    fs::write(&appimage, &bytes).expect("appimage");
    assert_eq!(identify(&appimage), Some(ArtifactFormat::AppImage));

    let text = dir.path().join("notes.txt");
    fs::write(&text, b"hello").expect("text");
    assert_eq!(identify(&text), None);
}

#[test]
fn survey_classifies_members_and_strips_the_shared_root() {
    let dir = TempDir::new().expect("tempdir");
    let archive = release_tarball(dir.path(), "delta-0.18.2-x86_64-unknown-linux-gnu");

    let report = survey(&archive).expect("survey should succeed");

    assert_eq!(report.format, ArtifactFormat::TarGz);
    assert_eq!(
        report.source_path,
        archive
            .canonicalize()
            .expect("canonical")
            .display()
            .to_string()
    );
    assert_eq!(report.strip_components, 1);
    assert_eq!(report.name.as_deref(), Some("delta"));
    assert_eq!(report.version.as_deref(), Some("0.18.2"));
    assert_eq!(report.architecture, "amd64");
    assert_eq!(report.sha256.len(), 64);

    let sole = report.sole_executable().expect("one launcher candidate");
    assert!(sole.path.ends_with("/delta"), "got {}", sole.path);

    assert_eq!(report.by_kind(ArtifactEntryKind::ManPage).len(), 1);
    assert_eq!(report.by_kind(ArtifactEntryKind::Completion).len(), 1);
    assert_eq!(report.by_kind(ArtifactEntryKind::License).len(), 1);
    assert_eq!(report.by_kind(ArtifactEntryKind::Doc).len(), 1);

    let dropped = report.dropped();
    assert_eq!(dropped.len(), 1, "windows artifact should be dropped");
    assert!(dropped[0].path.ends_with("delta.exe"));
}

#[test]
fn flat_archive_has_nothing_to_strip() {
    let dir = TempDir::new().expect("tempdir");
    let archive_path = dir.path().join("flat.tar");
    let file = fs::File::create(&archive_path).expect("archive");
    let mut builder = tar::Builder::new(file);

    let payload = b"#!/bin/sh\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(payload.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, "tool", &payload[..])
        .expect("append");
    builder.into_inner().expect("tar").sync_all().expect("sync");

    let report = survey(&archive_path).expect("survey");
    assert_eq!(report.strip_components, 0);
    assert_eq!(report.sole_executable().expect("launcher").path, "tool");
}

#[test]
fn foreign_members_do_not_choose_the_archive_root() {
    let dir = TempDir::new().expect("tempdir");
    let archive_path = dir.path().join("mixed.tar");
    let file = fs::File::create(&archive_path).expect("archive");
    let mut builder = tar::Builder::new(file);
    append_executable(&mut builder, "aaa-windows/tool.exe");
    append_executable(&mut builder, "zeta-1.0.0/bin/zeta");
    builder.into_inner().expect("tar").sync_all().expect("sync");

    let report = survey(&archive_path).expect("survey");
    assert_eq!(report.strip_components, 1);
    assert_eq!(report.name.as_deref(), Some("zeta"));
    assert_eq!(report.version.as_deref(), Some("1.0.0"));
}

#[test]
fn survey_limits_and_unsafe_paths_fail_closed() {
    assert!(checked_archive_path(std::path::Path::new("../escape")).is_err());
    assert!(checked_archive_path(std::path::Path::new("/absolute")).is_err());

    let mut entries = Vec::new();
    let mut unpacked = 0;
    assert!(
        push_entry(
            &mut entries,
            &mut unpacked,
            "huge".to_owned(),
            MAX_ARCHIVE_MEMBER_BYTES + 1,
            false,
        )
        .is_err()
    );

    unpacked = MAX_ARCHIVE_UNPACKED_BYTES;
    assert!(push_entry(&mut entries, &mut unpacked, "overflow".to_owned(), 1, false,).is_err());
}

#[test]
fn version_inference_only_accepts_dotted_numeric_segments() {
    assert!(looks_like_version("0.18.2"));
    assert!(looks_like_version("v1.2.3"));
    assert!(!looks_like_version("x86_64"));
    assert!(!looks_like_version("linux"));
    assert!(!looks_like_version("2"));

    assert_eq!(
        split_name_version("delta-0.18.2-x86_64-unknown-linux-gnu"),
        (Some("delta".to_owned()), Some("0.18.2".to_owned()))
    );
    assert_eq!(
        split_name_version("ripgrep-linux-x86_64"),
        (Some("ripgrep".to_owned()), None)
    );
}

#[test]
fn identity_falls_back_to_the_file_name_when_the_archive_is_flat() {
    assert_eq!(
        infer_identity(None, "delta-linux-x86_64.tar.gz"),
        (Some("delta".to_owned()), None)
    );
    assert_eq!(
        infer_identity(Some("delta-0.18.2-linux"), "whatever.tar.gz"),
        (Some("delta".to_owned()), Some("0.18.2".to_owned()))
    );
}

#[test]
fn library_and_foreign_members_are_not_launcher_candidates() {
    assert_eq!(
        classify("lib/libfoo.so.1", true),
        ArtifactEntryKind::Library
    );
    assert_eq!(
        classify("bin/tool.exe", true),
        ArtifactEntryKind::ForeignPlatform
    );
    assert_eq!(classify("bin/tool", true), ArtifactEntryKind::Executable);
    assert_eq!(
        classify("share/man/man1/tool.1", false),
        ArtifactEntryKind::ManPage
    );
    assert_eq!(classify("_tool", false), ArtifactEntryKind::Completion);
}

#[test]
fn elf_architecture_uses_header_metadata_instead_of_the_filename() {
    let mut header = [0u8; 20];
    header[..4].copy_from_slice(b"\x7fELF");
    header[4] = 2;
    header[5] = 1;
    header[18..20].copy_from_slice(&183u16.to_le_bytes());
    assert_eq!(elf_architecture(&header).expect("arm64 ELF"), "arm64");

    header[18..20].copy_from_slice(&0u16.to_le_bytes());
    assert!(elf_architecture(&header).is_err());
}

#[test]
fn survey_supports_bzip2_tar_and_zip_containers() {
    let dir = TempDir::new().expect("tempdir");

    let tar_bz2 = dir.path().join("tool-1.0.0.tar.bz2");
    let file = fs::File::create(&tar_bz2).expect("archive");
    let encoder = bzip2::write::BzEncoder::new(file, bzip2::Compression::fast());
    let mut builder = tar::Builder::new(encoder);
    append_executable(&mut builder, "tool-1.0.0/bin/tool");
    builder.into_inner().expect("tar").finish().expect("bzip2");

    let bzip_report = survey(&tar_bz2).expect("bzip2 tar survey");
    assert_eq!(bzip_report.format, ArtifactFormat::TarBz2);
    assert_eq!(
        bzip_report.sole_executable().expect("launcher").path,
        "tool-1.0.0/bin/tool"
    );

    let zip_path = dir.path().join("tool-1.0.0.zip");
    let file = fs::File::create(&zip_path).expect("zip");
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default().unix_permissions(0o755);
    archive
        .start_file("tool-1.0.0/bin/tool", options)
        .expect("zip entry");
    archive.write_all(b"#!/bin/sh\n").expect("zip body");
    archive.finish().expect("zip finish");

    let zip_report = survey(&zip_path).expect("zip survey");
    assert_eq!(zip_report.format, ArtifactFormat::Zip);
    assert_eq!(
        zip_report.sole_executable().expect("launcher").path,
        "tool-1.0.0/bin/tool"
    );
}

fn append_executable<W: Write>(builder: &mut tar::Builder<W>, path: &str) {
    let body = b"#!/bin/sh\n";
    let mut header = tar::Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, path, &body[..])
        .expect("append executable");
}
