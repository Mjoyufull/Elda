#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-repo",
    "Remote definitions, index sync, signature checks, and trust plumbing.",
);
