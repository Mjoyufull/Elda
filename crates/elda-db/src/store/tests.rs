use tempfile::TempDir;

use crate::layout::StateLayout;
use crate::schema::SCHEMA_VERSION;
use crate::store::Database;

#[test]
fn bootstrap_creates_layout_and_empty_state_files() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let layout = StateLayout::new(tempdir.path(), "/usr");
    let database = Database::new(layout.clone());

    let report = database.bootstrap().expect("bootstrap should succeed");

    assert_eq!(report.schema_version, SCHEMA_VERSION);
    assert!(layout.db_path.exists());
    assert!(layout.world_path.exists());
    assert!(layout.current_state_path.exists());
}

#[test]
fn read_only_bootstrap_creates_nothing_on_disk() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let layout = StateLayout::new(tempdir.path(), "/usr");
    let database = Database::read_only(layout.clone());

    let report = database
        .bootstrap()
        .expect("read-only bootstrap should succeed against an empty root");

    assert_eq!(report.schema_version, SCHEMA_VERSION);
    assert!(!report.created_database);
    assert!(
        !layout.db_path.exists(),
        "a query must not create the state database"
    );
    assert!(
        !layout.db_dir.exists(),
        "a query must not create the state layout"
    );
}

#[test]
fn read_only_queries_report_an_empty_root_without_writing() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let layout = StateLayout::new(tempdir.path(), "/usr");
    let database = Database::read_only(layout.clone());

    let packages = database
        .list_installed_packages()
        .expect("listing an empty root should succeed");

    assert!(packages.is_empty());
    assert!(!layout.db_path.exists());
}

#[test]
fn read_only_handle_sees_what_a_writable_handle_recorded() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let layout = StateLayout::new(tempdir.path(), "/usr");
    Database::new(layout.clone())
        .bootstrap()
        .expect("bootstrap should succeed");

    let reader = Database::read_only(layout);
    let report = reader.bootstrap().expect("read-only bootstrap");

    assert_eq!(report.schema_version, SCHEMA_VERSION);
    assert!(
        reader
            .list_installed_packages()
            .expect("query should succeed")
            .is_empty()
    );
}

#[test]
fn empty_database_reports_no_installed_packages() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let layout = StateLayout::new(tempdir.path(), "/usr");
    let database = Database::new(layout);
    database.bootstrap().expect("bootstrap should succeed");

    let packages = database
        .list_installed_packages()
        .expect("query should succeed");

    assert!(packages.is_empty());
}
