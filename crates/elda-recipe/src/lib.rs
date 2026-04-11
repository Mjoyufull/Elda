#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-recipe",
    "Recipe loading, schema validation, and legacy import seams.",
);
