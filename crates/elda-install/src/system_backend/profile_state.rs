use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use elda_db::{InstallationMode, StateLayout};

use crate::InstallError;

use super::{load_all_installed_system_metadata, reconcile_provider_assets};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileInitReconciliation {
    pub backend: String,
    pub previous_init: String,
    pub next_init: String,
    pub changed: bool,
    pub applied: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_provider_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct AppliedProfileState {
    #[serde(default)]
    init: String,
}

pub fn load_applied_profile_init(layout: &StateLayout) -> Result<String, InstallError> {
    if layout.mode != InstallationMode::System {
        return Ok(String::new());
    }

    Ok(load_applied_profile_state(layout)?.init)
}

pub fn plan_profile_init_reconciliation(
    layout: &StateLayout,
    desired_init: &str,
) -> Result<ProfileInitReconciliation, InstallError> {
    let desired_init = desired_init.trim().to_owned();

    Ok(ProfileInitReconciliation {
        backend: super::activation_backend_name(layout).to_owned(),
        previous_init: empty_to_unset(load_applied_profile_init(layout)?),
        next_init: empty_to_unset(desired_init.clone()),
        changed: load_applied_profile_init(layout)? != desired_init,
        applied: false,
        missing_provider_packages: missing_provider_packages(layout, &desired_init)?,
    })
}

pub fn reconcile_profile_init(
    layout: &StateLayout,
    desired_init: &str,
) -> Result<ProfileInitReconciliation, InstallError> {
    let mut report = plan_profile_init_reconciliation(layout, desired_init)?;
    if layout.mode != InstallationMode::System {
        return Ok(report);
    }

    let path = applied_profile_state_path(layout);
    let previous_bytes = if path.exists() {
        Some(fs::read(&path)?)
    } else {
        None
    };

    persist_applied_profile_state(layout, desired_init.trim())?;
    if let Err(error) = reconcile_provider_assets(layout) {
        restore_applied_profile_state(&path, previous_bytes.as_deref())?;
        return Err(error);
    }

    report.applied = true;
    Ok(report)
}

fn missing_provider_packages(
    layout: &StateLayout,
    desired_init: &str,
) -> Result<Vec<String>, InstallError> {
    if layout.mode != InstallationMode::System || desired_init.is_empty() {
        return Ok(Vec::new());
    }

    let mut packages = load_all_installed_system_metadata(layout)?
        .into_iter()
        .filter_map(|(package_name, metadata)| {
            let has_init_assets = metadata
                .provider_assets
                .iter()
                .any(|asset| asset.family == "init");
            let supports_provider = metadata
                .provider_assets
                .iter()
                .any(|asset| asset.family == "init" && asset.provider == desired_init);
            if has_init_assets && !supports_provider {
                Some(package_name)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    packages.sort();

    Ok(packages)
}

fn load_applied_profile_state(layout: &StateLayout) -> Result<AppliedProfileState, InstallError> {
    let path = applied_profile_state_path(layout);
    if !path.exists() {
        return Ok(AppliedProfileState::default());
    }

    Ok(serde_json::from_slice::<AppliedProfileState>(&fs::read(
        path,
    )?)?)
}

fn persist_applied_profile_state(
    layout: &StateLayout,
    desired_init: &str,
) -> Result<(), InstallError> {
    let path = applied_profile_state_path(layout);
    if desired_init.is_empty() {
        if path.exists() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&AppliedProfileState {
            init: desired_init.to_owned(),
        })?,
    )?;

    Ok(())
}

fn restore_applied_profile_state(
    path: &PathBuf,
    previous: Option<&[u8]>,
) -> Result<(), InstallError> {
    match previous {
        Some(bytes) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, bytes)?;
        }
        None if path.exists() => {
            fs::remove_file(path)?;
        }
        None => {}
    }

    Ok(())
}

fn applied_profile_state_path(layout: &StateLayout) -> PathBuf {
    layout
        .state_dir
        .join("system-backend")
        .join("profile-state.json")
}

fn empty_to_unset(value: String) -> String {
    if value.trim().is_empty() {
        "unset".to_owned()
    } else {
        value
    }
}
