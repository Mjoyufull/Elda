#![forbid(unsafe_code)]

mod error;
mod layout;
mod lock;
mod schema;
mod store;

pub use error::DbError;
pub use layout::{InstallationMode, StateLayout};
pub use store::{
    BootstrapReport, Database, HealthReport, InstallRecord, InstalledPackageDetails,
    InstalledPackageRecord, PackageDependencyRecord, PackageFileRecord, ReverseDependencyRecord,
    StateSnapshot,
};
