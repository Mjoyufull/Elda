mod architecture;
mod compare;
mod constraint;
#[cfg(test)]
mod constraint_tests;
mod identity;
mod version;

#[cfg(test)]
mod tests;

use thiserror::Error;

pub use architecture::Architecture;
pub use compare::compare_pkgver_strings;
pub use constraint::{
    ConstraintOperator, ConstraintParseError, ConstraintVersion, NamedConstraint,
};
pub use identity::PackageIdentity;
pub use version::PackageVersion;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PackageParseError {
    #[error("package identity is missing a name")]
    EmptyName,
    #[error("package identity is missing an architecture label")]
    EmptyArchitecture,
    #[error("package version is missing the pkgrel segment: {0}")]
    MissingPkgrel(String),
    #[error("package identity is missing the version segment: {0}")]
    MissingVersion(String),
    #[error("package version is missing pkgver")]
    EmptyPkgver,
    #[error("invalid epoch value: {0}")]
    InvalidEpoch(String),
    #[error("invalid pkgrel value: {0}")]
    InvalidPkgrel(String),
}
