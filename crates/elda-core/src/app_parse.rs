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
        let mut interactive = false;
        let mut query = None;

        for operand in &request.operands {
            if operand == "--regex" {
                regex = true;
                continue;
            }
            if operand == "--interactive" {
                interactive = true;
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

        Ok(ParsedSearchRequest {
            query,
            regex,
            interactive,
        })
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
        let mut git_ref = None;
        let mut values = Vec::new();
        let mut operands = request.operands.iter();

        while let Some(operand) = operands.next() {
            match operand.as_str() {
                "--to-tag" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-tag` requires one tag name".to_owned())
                    })?;
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Tag,
                        value,
                    )?;
                }
                "--to-rev" => {
                    let value = operands.next().ok_or_else(|| {
                        CoreError::Operator("`--to-rev` requires one revision".to_owned())
                    })?;
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Rev,
                        value,
                    )?;
                }
                _ if operand.starts_with("--to-tag=") => {
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Tag,
                        operand.trim_start_matches("--to-tag="),
                    )?;
                }
                _ if operand.starts_with("--to-rev=") => {
                    crate::app_install::set_git_ref(
                        &mut git_ref,
                        elda_recipe::GitRefKind::Rev,
                        operand.trim_start_matches("--to-rev="),
                    )?;
                }
                _ => values.push(operand.clone()),
            }
        }

        let mut values = values.into_iter();
        let package = values
            .next()
            .ok_or_else(|| CoreError::Operator("downgrade requires one package".to_owned()))?;
        let version = values
            .next()
            .map(|value| {
                PackageVersion::from_str(&value).map_err(|error| {
                    CoreError::Operator(format!("invalid downgrade version `{value}`: {error}"))
                })
            })
            .transpose()?;

        if let Some(extra) = values.next() {
            return Err(CoreError::Operator(format!(
                "unexpected downgrade operand `{extra}`"
            )));
        }
        if git_ref.is_some() && version.is_some() {
            return Err(CoreError::Operator(
                "source-ref downgrade flags cannot be combined with an archived version operand"
                    .to_owned(),
            ));
        }

        Ok(ParsedDowngradeRequest {
            package,
            version,
            git_ref,
        })
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
        let mut replace = false;
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
                "--replace" => {
                    replace = true;
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
            replace,
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

pub(crate) fn append_exclude_from_piece(piece: &str, excludes: &mut Vec<String>) {
    if piece.is_empty() {
        return;
    }

    if piece.contains(',') {
        for fragment in piece.split(',') {
            let fragment = fragment.trim();
            if !fragment.is_empty() {
                excludes.push(fragment.to_owned());
            }
        }
    } else {
        excludes.push(piece.trim().to_owned());
    }
}
