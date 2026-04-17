use std::fs;

use tempfile::TempDir;

use crate::journal::{JournalState, TransactionJournal};
use crate::recover_pending_transactions;
use elda_db::{Database, StateLayout};

#[test]
fn recover_removes_created_paths_from_incomplete_install_journal() {
    let tempdir = TempDir::new().expect("tempdir should exist");
    let layout = StateLayout::new(tempdir.path(), "/opt/elda");
    let database = Database::new(layout.clone());
    database.bootstrap().expect("bootstrap should succeed");

    let created_file = tempdir.path().join("opt/elda/bin/demo");
    fs::create_dir_all(created_file.parent().expect("parent should exist"))
        .expect("parent dir should exist");
    fs::write(&created_file, "demo").expect("created file should exist");

    let mut journal = TransactionJournal::new_install(
        "txn-test".to_owned(),
        "demo".to_owned(),
        layout.tmp_dir.join("transactions/test"),
    );
    journal.created_paths.push(created_file.clone());
    journal.state = JournalState::FilesApplied;
    journal.persist(&layout).expect("journal should persist");

    let report = recover_pending_transactions(&database).expect("recover should succeed");

    assert_eq!(report.recovered.len(), 1);
    assert!(!created_file.exists());
}
