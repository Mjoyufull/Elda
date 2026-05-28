use std::cmp::Ordering;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{PackageVersion, compare_pkgver_strings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConstraintOperator {
    Less,
    LessEqual,
    Equal,
    GreaterEqual,
    Greater,
    NotEqual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstraintVersion {
    pub epoch: u64,
    pub pkgver: String,
    pub pkgrel: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedConstraint {
    pub name: String,
    pub operator: Option<ConstraintOperator>,
    pub version: Option<ConstraintVersion>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConstraintParseError {
    #[error("constraint is empty")]
    Empty,
    #[error("constraint is missing a name: {0}")]
    MissingName(String),
    #[error("constraint has an unsupported operator: {0}")]
    InvalidOperator(String),
    #[error("constraint is missing a version: {0}")]
    MissingVersion(String),
    #[error("constraint version has an invalid epoch: {0}")]
    InvalidEpoch(String),
    #[error("constraint version is missing pkgver: {0}")]
    EmptyPkgver(String),
    #[error("constraint version has an invalid pkgrel: {0}")]
    InvalidPkgrel(String),
    #[error("provide constraint must use `=` when versioned: {0}")]
    InvalidProvideOperator(String),
}

impl NamedConstraint {
    pub fn parse_dependency(value: &str) -> Result<Self, ConstraintParseError> {
        parse_constraint(value, false)
    }

    pub fn parse_provide(value: &str) -> Result<Self, ConstraintParseError> {
        parse_constraint(value, true)
    }

    #[must_use]
    pub fn is_versioned(&self) -> bool {
        self.operator.is_some() && self.version.is_some()
    }

    #[must_use]
    pub fn matches_name(&self, candidate: &str) -> bool {
        self.name == candidate
    }

    #[must_use]
    pub fn matches_version(&self, actual: &ConstraintVersion) -> bool {
        match (self.operator, self.version.as_ref()) {
            (None, None) => true,
            (Some(operator), Some(version)) => version.matches(operator, actual),
            _ => false,
        }
    }
}

impl ConstraintVersion {
    #[must_use]
    pub fn from_parts(epoch: u64, pkgver: impl Into<String>, pkgrel: Option<u64>) -> Self {
        Self {
            epoch,
            pkgver: pkgver.into(),
            pkgrel,
        }
    }

    #[must_use]
    pub fn matches(&self, operator: ConstraintOperator, actual: &ConstraintVersion) -> bool {
        let ordering = compare_actual_to_requirement(actual, self);

        match operator {
            ConstraintOperator::Less => ordering == Ordering::Less,
            ConstraintOperator::LessEqual => ordering != Ordering::Greater,
            ConstraintOperator::Equal => ordering == Ordering::Equal,
            ConstraintOperator::GreaterEqual => ordering != Ordering::Less,
            ConstraintOperator::Greater => ordering == Ordering::Greater,
            ConstraintOperator::NotEqual => ordering != Ordering::Equal,
        }
    }
}

impl From<&PackageVersion> for ConstraintVersion {
    fn from(value: &PackageVersion) -> Self {
        Self {
            epoch: value.epoch,
            pkgver: value.pkgver.clone(),
            pkgrel: Some(value.pkgrel),
        }
    }
}

impl FromStr for ConstraintVersion {
    type Err = ConstraintParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value.is_empty() {
            return Err(ConstraintParseError::Empty);
        }

        let (epoch, remainder) = match value.split_once(':') {
            Some((epoch, remainder)) => (
                epoch
                    .parse::<u64>()
                    .map_err(|_| ConstraintParseError::InvalidEpoch(value.to_owned()))?,
                remainder,
            ),
            None => (0, value),
        };
        if remainder.trim().is_empty() {
            return Err(ConstraintParseError::EmptyPkgver(value.to_owned()));
        }

        let (pkgver, pkgrel) = match remainder.rsplit_once('-') {
            Some((pkgver, pkgrel))
                if !pkgver.trim().is_empty() && pkgrel.parse::<u64>().is_ok() =>
            {
                let pkgrel = pkgrel
                    .parse::<u64>()
                    .map_err(|_| ConstraintParseError::InvalidPkgrel(value.to_owned()))?;
                (pkgver.to_owned(), Some(pkgrel))
            }
            _ => (remainder.to_owned(), None),
        };
        if pkgver.trim().is_empty() {
            return Err(ConstraintParseError::EmptyPkgver(value.to_owned()));
        }

        Ok(Self {
            epoch,
            pkgver,
            pkgrel,
        })
    }
}

fn parse_constraint(
    value: &str,
    provide_mode: bool,
) -> Result<NamedConstraint, ConstraintParseError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ConstraintParseError::Empty);
    }

    let Some(operator_start) = value.find(['<', '>', '=', '!']) else {
        return Ok(NamedConstraint {
            name: value.to_owned(),
            operator: None,
            version: None,
        });
    };

    let name = value[..operator_start].trim();
    if name.is_empty() {
        return Err(ConstraintParseError::MissingName(value.to_owned()));
    }

    let tail = &value[operator_start..];
    let (operator, version_text) = parse_operator_and_version(tail, value)?;
    if provide_mode && operator != ConstraintOperator::Equal {
        return Err(ConstraintParseError::InvalidProvideOperator(
            value.to_owned(),
        ));
    }

    Ok(NamedConstraint {
        name: name.to_owned(),
        operator: Some(operator),
        version: Some(ConstraintVersion::from_str(version_text)?),
    })
}

fn parse_operator_and_version<'a>(
    tail: &'a str,
    raw: &str,
) -> Result<(ConstraintOperator, &'a str), ConstraintParseError> {
    let (operator, version_text) = if let Some(version) = tail.strip_prefix(">=") {
        (ConstraintOperator::GreaterEqual, version)
    } else if let Some(version) = tail.strip_prefix("<=") {
        (ConstraintOperator::LessEqual, version)
    } else if let Some(version) = tail.strip_prefix("!=") {
        (ConstraintOperator::NotEqual, version)
    } else if let Some(version) = tail.strip_prefix('=') {
        (ConstraintOperator::Equal, version)
    } else if let Some(version) = tail.strip_prefix('>') {
        (ConstraintOperator::Greater, version)
    } else if let Some(version) = tail.strip_prefix('<') {
        (ConstraintOperator::Less, version)
    } else {
        return Err(ConstraintParseError::InvalidOperator(raw.to_owned()));
    };

    let version_text = version_text.trim();
    if version_text.is_empty() {
        return Err(ConstraintParseError::MissingVersion(raw.to_owned()));
    }

    Ok((operator, version_text))
}

fn compare_actual_to_requirement(
    actual: &ConstraintVersion,
    requirement: &ConstraintVersion,
) -> Ordering {
    actual
        .epoch
        .cmp(&requirement.epoch)
        .then_with(|| compare_pkgver_strings(&actual.pkgver, &requirement.pkgver))
        .then_with(|| match requirement.pkgrel {
            Some(required_pkgrel) => actual.pkgrel.unwrap_or_default().cmp(&required_pkgrel),
            None => Ordering::Equal,
        })
}
