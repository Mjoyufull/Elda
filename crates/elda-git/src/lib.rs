#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-git",
    "Git source fetch, source inspection, and revision provenance.",
);
