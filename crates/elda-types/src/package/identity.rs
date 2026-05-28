use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::{Architecture, PackageParseError, PackageVersion};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageIdentity {
    pub name: String,
    pub arch: Option<Architecture>,
    pub version: PackageVersion,
}

impl Display for PackageIdentity {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.arch {
            Some(arch) => write!(formatter, "{}:{} {}", self.name, arch, self.version),
            None => write!(formatter, "{} {}", self.name, self.version),
        }
    }
}

impl FromStr for PackageIdentity {
    type Err = PackageParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let Some((name_part, version_part)) = value.split_once(' ') else {
            return Err(PackageParseError::MissingVersion(value.to_owned()));
        };
        if name_part.is_empty() {
            return Err(PackageParseError::EmptyName);
        }

        let (name, arch) = match name_part.split_once(':') {
            Some((name, arch)) => {
                if name.is_empty() {
                    return Err(PackageParseError::EmptyName);
                }
                (name.to_owned(), Some(Architecture::from_str(arch)?))
            }
            None => (name_part.to_owned(), None),
        };

        Ok(Self {
            name,
            arch,
            version: PackageVersion::from_str(version_part)?,
        })
    }
}
