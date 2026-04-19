#![forbid(unsafe_code)]

mod error;
mod index;
mod model;
mod store;

use elda_types::CrateBoundary;

pub use error::RepoError;
pub use index::{
    SyncOptions, load_remote_payload_trust, load_snapshot, resolve_package, search_packages,
    sync_remotes,
};
pub use model::{
    CacheDocument, RemoteDocument, RemotePayloadTrust, SyncReport, SyncedIndexSnapshot,
    SyncedPackageRecord, SyncedRemoteRecord, TrustMode, TrustedPublicKey,
};
pub use store::{
    add_cache, add_remote, list_caches, list_remotes, load_remote, save_cache, save_remote,
};

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-repo",
    "Remote and cache document management, index sync, and verification seams.",
);
