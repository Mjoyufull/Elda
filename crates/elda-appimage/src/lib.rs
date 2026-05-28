#![forbid(unsafe_code)]

mod desktop;
mod error;
mod inspect;
mod integration;
mod offset;

pub use error::AppImageError;
pub use inspect::{InspectReport, inspect_appimage};
pub use integration::{IntegrationOutcome, stage_integration_from_appimage};
pub use offset::squashfs_payload_offset;

use elda_types::CrateBoundary;

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-appimage",
    "Read-only Type 2 AppImage SquashFS integration without executing vendor payloads.",
);
