use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingTriggerRecord {
    pub name: String,
    pub reason: String,
    #[serde(default)]
    pub boot_path: bool,
    #[serde(default)]
    pub critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerRunRecord {
    pub name: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TriggerStateReport {
    pub(super) pending: Vec<PendingTriggerRecord>,
    pub(super) last_run: Vec<TriggerRunRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct BootStatusReport {
    #[serde(default)]
    pub managed_inputs: Vec<String>,
    #[serde(default)]
    pub pending_triggers: Vec<PendingTriggerRecord>,
    #[serde(default)]
    pub last_run: Vec<TriggerRunRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerRepairReport {
    pub repaired: Vec<String>,
    pub pending: Vec<PendingTriggerRecord>,
    pub backend: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_status: Option<BootStatusReport>,
}
