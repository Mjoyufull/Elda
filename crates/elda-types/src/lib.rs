#![forbid(unsafe_code)]

mod command;
mod package;

pub use command::{CommandReport, CrateBoundary, ExitStatus, NamespaceSpec, OutputMode};
pub use package::{
    Architecture, ConstraintOperator, ConstraintParseError, ConstraintVersion, NamedConstraint,
    PackageIdentity, PackageParseError, PackageVersion, compare_pkgver_strings,
};
