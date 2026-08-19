#![forbid(unsafe_code)]

mod artifact;
mod command;
mod package;

pub use artifact::{
    ArtifactAcquisition, ArtifactEntry, ArtifactEntryKind, ArtifactFormat, ArtifactSurvey,
};
pub use command::{CommandReport, CrateBoundary, ExitStatus, NamespaceSpec, OutputMode};
pub use package::{
    Architecture, ConstraintOperator, ConstraintParseError, ConstraintVersion, NamedConstraint,
    PackageIdentity, PackageParseError, PackageVersion, compare_pkgver_strings,
};
