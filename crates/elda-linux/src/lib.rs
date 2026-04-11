#![forbid(unsafe_code)]

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-linux",
    "Linux-only activation, multilib, and namespace implementations.",
);
