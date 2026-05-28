#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-ext",
    "Extension protocol, discovery, and adapter boundary plumbing.",
);
