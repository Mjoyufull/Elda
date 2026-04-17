mod model;
mod mutation;
mod query;
#[cfg(test)]
mod tests;

use crate::error::DbError;
use crate::layout::StateLayout;
use crate::lock::MutationLock;

pub use model::{
    BootstrapReport, HealthReport, InstallRecord, InstalledPackageDetails, InstalledPackageRecord,
    PackageDependencyRecord, PackageFileRecord, ReverseDependencyRecord, StateSnapshot,
};

#[derive(Debug, Clone)]
pub struct Database {
    layout: StateLayout,
}

impl Database {
    #[must_use]
    pub fn new(layout: StateLayout) -> Self {
        Self { layout }
    }

    #[must_use]
    pub fn layout(&self) -> &StateLayout {
        &self.layout
    }

    pub fn acquire_mutation_lock(&self) -> Result<MutationLock, DbError> {
        MutationLock::acquire(&self.layout.lock_path)
    }
}
