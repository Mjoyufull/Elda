use serde::Serialize;

use elda_db::{InstallationMode, StateLayout};
use elda_linux::activation_backend_for_system_mode;

use super::triggers::load_boot_status;
use crate::InstallError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActivationBackendStatus {
    pub name: String,
    pub capabilities: elda_linux::ActivationBackendCapabilities,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SystemBackendStatus {
    pub activation: ActivationBackendStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot: Option<super::BootStatusReport>,
}

pub fn load_system_backend_status(
    layout: &StateLayout,
) -> Result<SystemBackendStatus, InstallError> {
    let activation = activation_backend_status(layout);
    let boot = if layout.mode == InstallationMode::System {
        Some(load_boot_status(layout)?)
    } else {
        None
    };

    Ok(SystemBackendStatus { activation, boot })
}

#[must_use]
pub(crate) fn activation_backend_status(layout: &StateLayout) -> ActivationBackendStatus {
    let backend = activation_backend_for_system_mode(layout.mode == InstallationMode::System);
    ActivationBackendStatus {
        name: backend.name().to_owned(),
        capabilities: backend.capabilities(),
    }
}
