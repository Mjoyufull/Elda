#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

mod releases;
mod tags;

pub use releases::{
    AssetCompatibility, AssetFormat, AssetKind, GitReleaseAssetEntry, GitReleaseEntry,
    GitReleaseReport, GitReleaseSource, ReleaseInspectError, ReleaseProvider, ReleaseTarget,
    inspect_github_releases, inspect_releases, parse_github_repo_target, parse_release_target,
};
pub use tags::{
    GitInspectError, GitTagEntry, GitTagOptions, GitTagReport, VersionConfidence, list_remote_tags,
    list_remote_tags_with_options,
};

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-git",
    "Git source fetch, source inspection, and revision provenance.",
);
