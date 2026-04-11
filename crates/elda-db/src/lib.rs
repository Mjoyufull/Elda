#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-db",
    "SQLite package DB, manifests, journals, and world tracking.",
);
