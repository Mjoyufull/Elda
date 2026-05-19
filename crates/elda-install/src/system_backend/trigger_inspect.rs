use std::fs;

use serde::Serialize;

use elda_db::{InstallationMode, StateLayout};
use elda_linux::activation_backend_for_system_mode;

use crate::InstallError;

use super::trigger_model::{BootStatusReport, PendingTriggerRecord, TriggerRunRecord};
use super::triggers::{
    load_boot_status, load_trigger_state, trigger_from_name, trigger_output_path,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerInspectionReport {
    pub backend: String,
    pub system_mode: bool,
    pub pending: Vec<PendingTriggerRecord>,
    pub last_run: Vec<TriggerRunRecord>,
    pub boot_status: BootStatusReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerDetailReport {
    pub backend: String,
    pub system_mode: bool,
    pub name: String,
    pub known: bool,
    pub boot_path: bool,
    pub critical: bool,
    pub pending: Option<PendingTriggerRecord>,
    pub last_run: Option<TriggerRunRecord>,
    pub output_path: String,
    pub output: Option<serde_json::Value>,
}

pub fn inspect_triggers(layout: &StateLayout) -> Result<TriggerInspectionReport, InstallError> {
    let state = load_trigger_state(layout)?;

    Ok(TriggerInspectionReport {
        backend: backend_name(layout),
        system_mode: layout.mode == InstallationMode::System,
        pending: state.pending,
        last_run: state.last_run,
        boot_status: load_boot_status(layout)?,
    })
}

pub fn inspect_trigger(
    layout: &StateLayout,
    name: &str,
) -> Result<TriggerDetailReport, InstallError> {
    let state = load_trigger_state(layout)?;
    let trigger = trigger_from_name(name);
    let output_path = trigger
        .map(|trigger| trigger_output_path(layout, trigger))
        .unwrap_or_else(|| {
            layout
                .state_dir
                .join("system-backend/triggers")
                .join(format!("{name}.json"))
        });
    let output = if output_path.exists() {
        Some(serde_json::from_slice(&fs::read(&output_path)?)?)
    } else {
        None
    };

    Ok(TriggerDetailReport {
        backend: backend_name(layout),
        system_mode: layout.mode == InstallationMode::System,
        name: name.to_owned(),
        known: trigger.is_some(),
        boot_path: trigger.is_some_and(|trigger| trigger.is_boot_trigger()),
        critical: trigger.is_some_and(|trigger| trigger.is_critical()),
        pending: state
            .pending
            .iter()
            .find(|record| record.name == name)
            .cloned(),
        last_run: state
            .last_run
            .iter()
            .find(|record| record.name == name)
            .cloned(),
        output_path: output_path.display().to_string(),
        output,
    })
}

fn backend_name(layout: &StateLayout) -> String {
    activation_backend_for_system_mode(layout.mode == InstallationMode::System)
        .name()
        .to_owned()
}
