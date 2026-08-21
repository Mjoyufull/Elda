use std::io::Write;

use tar::{Builder, Header};

use super::support::*;
use super::*;

#[test]
fn metadata_add_local_artifact_records_inventory_and_installs_from_canonical_path() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    write_prefix_config(tempdir.path(), "/opt/elda");
    let archive = tempdir.path().join("artifact-tool-1.2.3.tar");
    write_release_archive(&archive);

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["a".to_owned()],
            vec![archive.display().to_string()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("local artifact metadata add should succeed");

    let target = &report.details.as_ref().expect("details")["metadata_add"]["targets"][0];
    assert_eq!(target["recipe_name"], "artifact-tool");
    assert_eq!(target["selected_source_kind"], "url_archive");
    assert_eq!(target["artifact_survey"]["format"], "tar");
    assert_eq!(
        target["artifact_survey"]["entries"]
            .as_array()
            .expect("entries")
            .len(),
        2
    );

    let pkg_lua = fs::read_to_string(
        tempdir
            .path()
            .join("etc/elda/recipes/artifact-tool/pkg.lua"),
    )
    .expect("generated pkg.lua should read");
    assert!(
        pkg_lua.contains(&format!("url = \"file://{}\"", archive.display())),
        "generated local source must retain its canonical path: {pkg_lua}"
    );
    assert!(pkg_lua.contains("strip_components = 1"));
    assert!(pkg_lua.contains("binary = \"bin/artifact-tool\""));
    assert!(
        tempdir
            .path()
            .join("etc/elda/recipes/artifact-tool/artifact-survey.json")
            .is_file()
    );

    run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["i".to_owned()],
            vec!["artifact-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("generated local artifact recipe should install");

    assert_eq!(
        run_installed_binary(tempdir.path(), "/opt/elda/bin/artifact-tool"),
        "local artifact"
    );
}

fn write_release_archive(path: &std::path::Path) {
    let file = fs::File::create(path).expect("archive should create");
    let mut builder = Builder::new(file);
    append_file(
        &mut builder,
        "artifact-tool-1.2.3/bin/artifact-tool",
        0o755,
        b"#!/bin/sh\necho local artifact\n",
    );
    append_file(
        &mut builder,
        "artifact-tool-1.2.3/README.md",
        0o644,
        b"# Artifact Tool\n",
    );
    builder.finish().expect("archive should finish");
    builder
        .into_inner()
        .expect("archive should flush")
        .flush()
        .expect("archive file should flush");
}

fn append_file(builder: &mut Builder<fs::File>, path: &str, mode: u32, body: &[u8]) {
    let mut header = Header::new_gnu();
    header.set_size(body.len() as u64);
    header.set_mode(mode);
    header.set_cksum();
    builder
        .append_data(&mut header, path, body)
        .expect("archive member should append");
}
