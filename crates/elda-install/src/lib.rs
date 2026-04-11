#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-install",
    "Conflict checks, global mutation lock, and transaction execution.",
);
