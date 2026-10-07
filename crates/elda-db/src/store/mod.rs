mod model;
mod mutation;
mod query;
#[cfg(test)]
mod tests;

use rusqlite::{Connection, OpenFlags};

use crate::error::DbError;
use crate::layout::StateLayout;
use crate::lock::MutationLock;
use crate::schema;

pub use model::{
    BootstrapReport, HealthReport, InstallRecord, InstalledPackageDetails, InstalledPackageRecord,
    PackageDependencyRecord, PackageFileRecord, ReverseDependencyRecord, StateSnapshot,
};

#[derive(Debug, Clone)]
pub struct Database {
    layout: StateLayout,
    read_only: bool,
}

impl Database {
    #[must_use]
    pub fn new(layout: StateLayout) -> Self {
        Self {
            layout,
            read_only: false,
        }
    }

    /// A handle that never writes to disk.
    ///
    /// Query commands use this so that reading the installed set cannot create
    /// directories, create the database file, or take the mutation lock — and
    /// therefore never needs privilege it should not have.
    #[must_use]
    pub fn read_only(layout: StateLayout) -> Self {
        Self {
            layout,
            read_only: true,
        }
    }

    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    #[must_use]
    pub fn layout(&self) -> &StateLayout {
        &self.layout
    }

    pub fn acquire_mutation_lock(&self) -> Result<MutationLock, DbError> {
        self.require_writable()?;
        MutationLock::acquire(&self.layout.lock_path)
    }

    fn require_writable(&self) -> Result<(), DbError> {
        if self.read_only {
            Err(DbError::ReadOnly)
        } else {
            Ok(())
        }
    }

    /// Open a connection honouring the handle's write posture.
    ///
    /// In read-only mode a missing state database is not an error: it means the
    /// root has nothing installed, so we answer from an empty in-memory schema
    /// rather than creating one on disk.
    pub(crate) fn connect(&self) -> Result<Connection, DbError> {
        if !self.read_only {
            return Ok(Connection::open(&self.layout.db_path)?);
        }
        if !self.layout.db_path.try_exists()? {
            let connection = Connection::open_in_memory()?;
            schema::initialize(&connection)?;
            return Ok(connection);
        }
        let connection =
            Connection::open_with_flags(&self.layout.db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        if schema::current_version(&connection)? < schema::SCHEMA_VERSION {
            // Migrate a consistent private snapshot, never the operator's database.
            let mut snapshot = Connection::open_in_memory()?;
            {
                let backup = rusqlite::backup::Backup::new(&connection, &mut snapshot)?;
                if backup.step(-1)? != rusqlite::backup::StepResult::Done {
                    return Err(rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
                        None,
                    )
                    .into());
                }
            }
            schema::initialize(&snapshot)?;
            return Ok(snapshot);
        }
        Ok(connection)
    }
}
