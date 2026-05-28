use std::fs;
use std::path::Path;

use super::*;
use elda_types::ExitStatus;

#[test]
fn review_ls_info_forget_and_diff_round_trip() {
    let tempdir = tempfile::TempDir::new().expect("tempdir should be created");
    let data_dir = tempdir.path().join("var/lib/elda");
    let recipes_dir = tempdir.path().join("etc/elda/recipes");
    fs::create_dir_all(recipes_dir.join("demo")).expect("recipe dir should exist");
    let recipe_path = recipes_dir.join("demo/pkg.lua");
    fs::write(&recipe_path, "return {}").expect("recipe should be written");

    crate::app_review_memory::write_review_stamp(&data_dir, "demo", "interbuild", &recipe_path)
        .expect("stamp should be written");

    let list = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["review".to_owned(), "ls".to_owned()],
            Vec::new(),
            OutputMode::Json,
            false,
        ),
    )
    .expect("review ls should succeed");
    assert_eq!(list.area, "review");
    assert_eq!(list.exit_status, ExitStatus::Success);
    assert!(
        list.details
            .as_ref()
            .and_then(|details| details.get("stamps"))
            .and_then(|stamps| stamps.as_array())
            .is_some_and(|stamps| !stamps.is_empty())
    );

    let info = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["review".to_owned(), "info".to_owned()],
            vec!["demo".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("review info should succeed");
    assert_eq!(info.status, "ok");

    let diff = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["review".to_owned(), "diff".to_owned()],
            vec![
                "demo".to_owned(),
                "--kind".to_owned(),
                "interbuild".to_owned(),
            ],
            OutputMode::Json,
            true,
        ),
    )
    .expect("review diff dry-run should succeed");
    assert_eq!(diff.status, "current");

    let forget = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["review".to_owned(), "forget".to_owned()],
            vec![
                "demo".to_owned(),
                "--kind".to_owned(),
                "interbuild".to_owned(),
            ],
            OutputMode::Json,
            false,
        ),
    )
    .expect("review forget should succeed");
    assert_eq!(forget.status, "ok");
    assert!(
        !stamp_path(&data_dir, "demo", "interbuild").exists(),
        "stamp file should be removed"
    );
}

fn stamp_path(data_dir: &Path, package: &str, review_kind: &str) -> std::path::PathBuf {
    data_dir
        .join("review-stamps")
        .join(review_kind)
        .join(format!("{package}.json"))
}
