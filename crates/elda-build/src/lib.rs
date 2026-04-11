#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-build",
    "Build orchestration, staging roots, and payload assembly.",
);
