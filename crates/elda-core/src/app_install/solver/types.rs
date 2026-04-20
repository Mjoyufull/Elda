use std::fmt::{Display, Formatter};

use elda_types::PackageVersion;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SolverVersion {
    Absent,
    Choice(u32),
    Package(PackageVersion),
    Root,
}

impl Display for SolverVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Absent => write!(formatter, "absent"),
            Self::Choice(value) => write!(formatter, "choice-{value}"),
            Self::Package(version) => write!(formatter, "{version}"),
            Self::Root => write!(formatter, "root"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum SolverPackage {
    Root,
    Real(String),
    Choice(String),
}

impl Display for SolverPackage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root => write!(formatter, "root"),
            Self::Real(package_name) => write!(formatter, "{package_name}"),
            Self::Choice(key) => write!(formatter, "{key}"),
        }
    }
}
