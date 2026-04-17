use std::str::FromStr;

use crate::CommandRequest;
use crate::app::{
    ParsedDiffRequest, ParsedDowngradeRequest, ParsedHoldRequest, ParsedRdepsRequest,
    ParsedRemoveRequest, ParsedSearchRequest, ParsedVendorAddRequest,
};
use crate::error::CoreError;

use elda_db::InstalledPackageDetails;
use elda_types::PackageVersion;

use crate::app::AppContext;

impl AppContext {
    pub(crate) fn parse_search_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedSearchRequest, CoreError> {
        let mut regex = false;
        let mut query = None;

        for operand in &request.operands {
            if operand == "--regex" {
                regex = true;
                continue;
            }
            if query.is_some() {
                return Err(CoreError::Operator(
                    "search accepts exactly one query operand".to_owned(),
                ));
            }
            query = Some(operand.clone());
        }

        let query = query.ok_or_else(|| {
            CoreError::Operator("search requires a substring or regex query".to_owned())
        })?;

        Ok(ParsedSearchRequest { query, regex })
    }

    pub(crate) fn parse_remove_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedRemoveRequest, CoreError> {
        let mut packages = Vec::new();
        let mut cascade = false;
        let mut purge_conffiles = false;

        for operand in &request.operands {
            match operand.as_str() {
                "--cascade" => cascade = true,
                "--purge-conffiles" => purge_conffiles = true,
                _ => packages.push(operand.clone()),
            }
        }

        if packages.is_empty() {
            return Err(CoreError::Operator(
                "remove requires at least one installed package name".to_owned(),
            ));
        }

        Ok(ParsedRemoveRequest {
            packages,
            cascade,
            purge_conffiles,
        })
    }

    pub(crate) fn parse_diff_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedDiffRequest, CoreError> {
        let mut candidate = false;
        let mut package = None;

        for operand in &request.operands {
            if operand == "--candidate" {
                candidate = true;
                continue;
            }
            if package.is_some() {
                return Err(CoreError::Operator(
                    "diff accepts one package operand".to_owned(),
                ));
            }
            package = Some(operand.clone());
        }

        Ok(ParsedDiffRequest {
            package: package
                .ok_or_else(|| CoreError::Operator("diff requires one package".to_owned()))?,
            candidate,
        })
    }

    pub(crate) fn parse_downgrade_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedDowngradeRequest, CoreError> {
        let mut operands = request.operands.iter();
        let package = operands
            .next()
            .ok_or_else(|| CoreError::Operator("downgrade requires one package".to_owned()))?
            .clone();
        let version = operands
            .next()
            .map(|value| {
                PackageVersion::from_str(value).map_err(|error| {
                    CoreError::Operator(format!("invalid downgrade version `{value}`: {error}"))
                })
            })
            .transpose()?;

        if let Some(extra) = operands.next() {
            return Err(CoreError::Operator(format!(
                "unexpected downgrade operand `{extra}`"
            )));
        }

        Ok(ParsedDowngradeRequest { package, version })
    }

    pub(crate) fn parse_rdeps_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedRdepsRequest, CoreError> {
        let mut package = None;
        let mut recursive = false;
        let mut include_weak = false;

        for operand in &request.operands {
            match operand.as_str() {
                "--all" => recursive = true,
                "--weak" => include_weak = true,
                _ => {
                    if package.is_some() {
                        return Err(CoreError::Operator(
                            "rdeps accepts one package operand".to_owned(),
                        ));
                    }
                    package = Some(operand.clone());
                }
            }
        }

        Ok(ParsedRdepsRequest {
            package: package
                .ok_or_else(|| CoreError::Operator("rdeps requires one package".to_owned()))?,
            recursive,
            include_weak,
        })
    }

    pub(crate) fn parse_hold_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedHoldRequest, CoreError> {
        let mut package = None;
        let mut source = None;
        let mut operands = request.operands.iter();

        while let Some(operand) = operands.next() {
            if operand == "--source" {
                let value = operands.next().ok_or_else(|| {
                    CoreError::Operator("hold `--source` requires a value".to_owned())
                })?;
                source = Some(value.clone());
                continue;
            }

            if package.is_some() {
                return Err(CoreError::Operator(
                    "hold accepts one package operand".to_owned(),
                ));
            }
            package = Some(operand.clone());
        }

        Ok(ParsedHoldRequest {
            package: package
                .ok_or_else(|| CoreError::Operator("hold requires one package".to_owned()))?,
            source,
        })
    }

    pub(crate) fn parse_vendor_add_request(
        &self,
        request: &CommandRequest,
    ) -> Result<ParsedVendorAddRequest, CoreError> {
        let mut binary = None;
        let mut asset = None;
        let mut positionals = Vec::new();
        let mut operands = request.operands.iter();

        while let Some(operand) = operands.next() {
            match operand.as_str() {
                "--binary" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("vendor add `--binary` requires a value".to_owned())
                    })?;
                    binary = Some(value.clone());
                }
                "--asset" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("vendor add `--asset` requires a value".to_owned())
                    })?;
                    asset = Some(value.clone());
                }
                _ => positionals.push(operand.clone()),
            }
        }

        if positionals.len() != 2 {
            return Err(CoreError::Operator(
                "vendor add requires `<pkg> <source>`".to_owned(),
            ));
        }

        Ok(ParsedVendorAddRequest {
            package_name: positionals[0].clone(),
            source: positionals[1].clone(),
            binary,
            asset,
        })
    }
}

#[must_use]
pub(crate) fn dependency_name_from_constraint(constraint: &str) -> String {
    constraint.find(['<', '>', '=', '!']).map_or_else(
        || constraint.trim().to_owned(),
        |index| constraint[..index].trim().to_owned(),
    )
}

#[must_use]
pub(crate) fn installed_version(package: &InstalledPackageDetails) -> String {
    format!("{}:{}-{}", package.epoch, package.pkgver, package.pkgrel)
}
