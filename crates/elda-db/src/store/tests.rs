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
