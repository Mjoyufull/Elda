use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::InstallError;
use crate::fsops::{cleanup_paths, restore_backups};
use crate::snapshot::SnapshotRecord;
use elda_db::StateLayout;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JournalState {
    Prepared,
    FilesApplied,
    DbCommitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionKind {
    Install,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupEntry {
    pub original_path: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub path_kind: String,
    pub link_target: Option<String>,
    pub mode: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionJournal {
    pub journal_id: String,
    pub package_name: String,
    pub transaction_kind: TransactionKind,
    pub state: JournalState,
    pub transaction_root: PathBuf,
    pub state_id: Option<String>,
    pub created_paths: Vec<PathBuf>,
    pub backup_entries: Vec<BackupEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snapshots: Vec<SnapshotRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingJournal {
    pub path: PathBuf,
    pub journal: TransactionJournal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecoveryReport {
    pub recovered: Vec<RecoveredJournal>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecoveredJournal {
    pub journal_id: String,
    pub package_name: String,
    pub action: String,
    pub prior_state: JournalState,
}

impl TransactionJournal {
    pub fn new_install(
        journal_id: String,
        package_name: String,
        transaction_root: PathBuf,
    ) -> Self {
        Self {
            journal_id,
            package_name,
            transaction_kind: TransactionKind::Install,
            state: JournalState::Prepared,
            transaction_root,
            state_id: None,
            created_paths: Vec::new(),
            backup_entries: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn new_remove(journal_id: String, package_name: String, transaction_root: PathBuf) -> Self {
        Self {
            journal_id,
            package_name,
            transaction_kind: TransactionKind::Remove,
            state: JournalState::Prepared,
            transaction_root,
            state_id: None,
            created_paths: Vec::new(),
            backup_entries: Vec::new(),
            snapshots: Vec::new(),
        }
    }

    pub fn path(&self, layout: &StateLayout) -> PathBuf {
        layout.journal_dir.join(format!("{}.json", self.journal_id))
    }

    pub fn persist(&self, layout: &StateLayout) -> Result<(), InstallError> {
        fs::create_dir_all(&layout.journal_dir)?;
        fs::write(self.path(layout), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn remove(self, layout: &StateLayout) -> Result<(), InstallError> {
        let path = self.path(layout);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

pub fn ensure_no_pending_journals(layout: &StateLayout) -> Result<(), InstallError> {
    let journals = pending_journals(layout)?;
    if journals.is_empty() {
        Ok(())
    } else {
        Err(InstallError::PendingRecovery(journals.len()))
    }
}

pub fn recover_pending_journals(layout: &StateLayout) -> Result<RecoveryReport, InstallError> {
    let journals = pending_journals(layout)?;
    let mut recovered = Vec::with_capacity(journals.len());

    for pending in journals {
        let prior_state = pending.journal.state;
        let action = match (pending.journal.transaction_kind, pending.journal.state) {
            (TransactionKind::Install, JournalState::DbCommitted)
            | (TransactionKind::Remove, JournalState::DbCommitted) => {
                cleanup_transaction_root(&pending.journal.transaction_root)?;
                "finalized-committed".to_owned()
            }
            (TransactionKind::Install, _) => {
                cleanup_paths(&pending.journal.created_paths)?;
                restore_backups(&pending.journal.backup_entries)?;
                cleanup_transaction_root(&pending.journal.transaction_root)?;
                "rolled-back-install".to_owned()
            }
            (TransactionKind::Remove, _) => {
                cleanup_paths(&pending.journal.created_paths)?;
                restore_backups(&pending.journal.backup_entries)?;
                cleanup_transaction_root(&pending.journal.transaction_root)?;
                "rolled-back-remove".to_owned()
            }
        };

        if pending.path.exists() {
            fs::remove_file(&pending.path)?;
        }
        recovered.push(RecoveredJournal {
            journal_id: pending.journal.journal_id,
            package_name: pending.journal.package_name,
            action,
            prior_state,
        });
    }

    Ok(RecoveryReport { recovered })
}

fn pending_journals(layout: &StateLayout) -> Result<Vec<PendingJournal>, InstallError> {
    if !layout.journal_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&layout.journal_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    let mut journals = Vec::new();
    for path in entries {
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read(&path)?;
        let journal = serde_json::from_slice::<TransactionJournal>(&content).map_err(|error| {
            InstallError::Journal(format!("failed to parse {}: {error}", path.display()))
        })?;
        journals.push(PendingJournal { path, journal });
    }

    Ok(journals)
}

fn cleanup_transaction_root(path: &Path) -> Result<(), InstallError> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}
