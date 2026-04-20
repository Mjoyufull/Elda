mod apply;
mod coherence;
mod decision;
mod downgrade;
mod parse;
mod plan;

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use serde_json::json;

use crate::app::{AppContext, ParsedInstallRequest, PlannedUpgradeAction, UpgradeDecision};
use crate::app_parse::installed_version;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};
use elda_install::{install_built_package, install_upgraded_package, remove_package_for_upgrade};
use elda_types::PackageVersion;
