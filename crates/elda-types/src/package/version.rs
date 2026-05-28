use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::PackageParseError;
use super::compare::compare_pkgver_strings;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageVersion {
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: u64,
}

impl Display for PackageVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}-{}", self.epoch, self.pkgver, self.pkgrel)
    }
}

impl FromStr for PackageVersion {
    type Err = PackageParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (epoch, remainder) = match value.split_once(':') {
            Some((epoch, remainder)) => (
                epoch
                    .parse::<u64>()
                    .map_err(|_| PackageParseError::InvalidEpoch(epoch.to_owned()))?,
                remainder,
            ),
            None => (0, value),
        };

        let Some((pkgver, pkgrel)) = remainder.rsplit_once('-') else {
            return Err(PackageParseError::MissingPkgrel(value.to_owned()));
        };
        if pkgver.is_empty() {
            return Err(PackageParseError::EmptyPkgver);
        }

        let pkgrel = pkgrel
            .parse::<u64>()
            .map_err(|_| PackageParseError::InvalidPkgrel(pkgrel.to_owned()))?;

        Ok(Self {
            epoch,
            pkgver: pkgver.to_owned(),
            pkgrel,
        })
    }
}

impl Ord for PackageVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.epoch
            .cmp(&other.epoch)
            .then_with(|| compare_pkgver_strings(&self.pkgver, &other.pkgver))
            .then_with(|| self.pkgrel.cmp(&other.pkgrel))
    }
}

impl PartialOrd for PackageVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
