use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::PackageParseError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Architecture {
    Amd64,
    I386,
    Arm64,
    Armhf,
    Riscv64,
    Ppc64le,
    Other(String),
}

impl Display for Architecture {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Amd64 => formatter.write_str("amd64"),
            Self::I386 => formatter.write_str("i386"),
            Self::Arm64 => formatter.write_str("arm64"),
            Self::Armhf => formatter.write_str("armhf"),
            Self::Riscv64 => formatter.write_str("riscv64"),
            Self::Ppc64le => formatter.write_str("ppc64le"),
            Self::Other(label) => formatter.write_str(label),
        }
    }
}

impl FromStr for Architecture {
    type Err = PackageParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "amd64" => Ok(Self::Amd64),
            "i386" => Ok(Self::I386),
            "arm64" => Ok(Self::Arm64),
            "armhf" => Ok(Self::Armhf),
            "riscv64" => Ok(Self::Riscv64),
            "ppc64le" => Ok(Self::Ppc64le),
            "" => Err(PackageParseError::EmptyArchitecture),
            other => Ok(Self::Other(other.to_owned())),
        }
    }
}
