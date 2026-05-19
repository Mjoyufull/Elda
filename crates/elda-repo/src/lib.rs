#![forbid(unsafe_code)]

mod error;
mod index;
mod model;
mod store;

use elda_types::CrateBoundary;

pub use error::RepoError;
pub use index::{
    RemoteSyncEvent, SyncOptions, inspect_remote_trust, load_remote_payload_trust, load_snapshot,
    preview_interemote, preview_interemote_with_protocols, resolve_package, search_packages,
    sync_remotes,
};
pub use model::{
    CacheDocument, DEFAULT_REMOTE_CHANNEL, InteremotePreview, InteremotePreviewPackage,
    RemoteDocument, RemotePayloadTrust, RemoteTrustInspection, SyncPackageDelta, SyncReport,
    SyncedIndexSnapshot, SyncedPackageRecord, SyncedRemoteRecord, TrustMode, TrustedPublicKey,
};
pub use store::{
    add_cache, add_remote, list_caches, list_remotes, load_remote, remove_remote, save_cache,
    save_remote,
};

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-repo",
    "Remote and cache document management, index sync, and verification seams.",
);
