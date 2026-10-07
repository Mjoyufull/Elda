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

    rusqlite::Connection::open(&layout.db_path)
        .expect("connection")
        .execute(
            "INSERT INTO installed_packages (pkgname, pkgver, pkgrel) VALUES ('example', '1.0', 1)",
            [],
        )
        .expect("record package");

    let reader = Database::read_only(layout);
    let report = reader.bootstrap().expect("read-only bootstrap");

    assert_eq!(report.schema_version, SCHEMA_VERSION);
    assert_eq!(
        reader
            .list_installed_packages()
            .expect("query should succeed")[0]
            .pkgname,
        "example"
    );
}

#[test]
fn legacy_read_only_queries_migrate_only_a_private_snapshot() {
    for version in [1, 2] {
        let tempdir = TempDir::new().expect("tempdir");
        let layout = StateLayout::new(tempdir.path(), "/usr");
        Database::new(layout.clone())
            .bootstrap()
            .expect("bootstrap");
        let connection = rusqlite::Connection::open(&layout.db_path).expect("connection");
        connection
            .execute_batch("ALTER TABLE package_files DROP COLUMN is_conffile;")
            .expect("v2 schema");
        if version == 1 {
            connection
                .execute_batch(
                    "ALTER TABLE installed_packages DROP COLUMN pinned_version;
                ALTER TABLE installed_packages DROP COLUMN held;
                ALTER TABLE installed_packages DROP COLUMN hold_source;",
                )
                .expect("v1 schema");
        }
        connection
            .execute("UPDATE schema_meta SET schema_version = ?", [version])
            .expect("version");
        connection
            .pragma_update(None, "user_version", version)
            .expect("schema version");
        connection.execute("INSERT INTO installed_packages (pkgname, pkgver, pkgrel) VALUES ('example', '1', 1)", []).expect("package");
        connection.execute("INSERT INTO package_files (pkgname, path, path_kind) VALUES ('example', 'bin/example', 'file')", []).expect("file");
        drop(connection);
        let original = std::fs::read(&layout.db_path).expect("original database");
        let reader = Database::read_only(layout.clone());
        assert_eq!(
            reader.list_installed_packages().expect("list")[0].pkgname,
            "example"
        );
        assert!(
            !reader
                .installed_package("example")
                .expect("details")
                .expect("package")
                .held
        );
        assert!(!reader.package_files("example").expect("files")[0].is_conffile);
        assert_eq!(std::fs::read(&layout.db_path).expect("database"), original);
    }
}

#[test]
fn read_only_mutations_are_rejected_before_creating_files() {
    let tempdir = TempDir::new().expect("tempdir");
    let layout = StateLayout::new(tempdir.path(), "/usr");
    let reader = Database::read_only(layout.clone());
    assert!(matches!(
        reader.acquire_mutation_lock(),
        Err(crate::error::DbError::ReadOnly)
    ));
    for result in [
        reader.remove_package("example"),
        reader.set_current_state("state"),
        reader.set_install_reason("example", "explicit"),
        reader.set_pinned_version("example", None),
        reader.set_hold("example", true, None),
    ] {
        assert!(matches!(result, Err(crate::error::DbError::ReadOnly)));
    }
    assert!(!layout.data_dir.exists());
}

#[test]
fn initialized_roots_require_companion_state_files() {
    for missing in ["world", "current"] {
        let tempdir = TempDir::new().expect("tempdir");
        let layout = StateLayout::new(tempdir.path(), "/usr");
        Database::new(layout.clone())
            .bootstrap()
            .expect("bootstrap");
        std::fs::remove_file(if missing == "world" {
            &layout.world_path
        } else {
            &layout.current_state_path
        })
        .expect("remove");
        assert!(Database::read_only(layout).state_snapshot().is_err());
    }
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
