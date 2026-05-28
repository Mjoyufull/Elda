#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-unix",
    "Unix backend traits for activation, build execution, and analysis.",
);
