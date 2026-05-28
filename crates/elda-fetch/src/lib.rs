#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-fetch",
    "HTTP fetch, cache reads, and checksum verification primitives.",
);
