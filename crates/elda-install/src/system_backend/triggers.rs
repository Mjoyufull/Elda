use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::json;

use elda_build::BuiltPackage;
use elda_db::{Database, InstallationMode, StateLayout};
use elda_linux::{SystemTrigger, detect_trigger_names};

use crate::InstallError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingTriggerRecord {
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TriggerRunRecord {
    name: String,
    output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct TriggerState {
    pending: Vec<PendingTriggerRecord>,
    last_run: Vec<TriggerRunRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TriggerRepairReport {
    pub repaired: Vec<String>,
    pub pending: Vec<PendingTriggerRecord>,
    pub backend: String,
}

pub(crate) fn run_install_triggers(
    database: &Database,
    package: &BuiltPackage,
) -> Result<(), InstallError> {
    let trigger_names = detect_trigger_names(
        package
            .manifest
            .entries
            .iter()
            .map(|entry| entry.path.as_str()),
    );
    run_triggers(database, &trigger_names)
}

pub(crate) fn run_remove_triggers(
    database: &Database,
    removed_paths: &[String],
) -> Result<(), InstallError> {
    let trigger_names = detect_trigger_names(removed_paths.iter().map(String::as_str));
    run_triggers(database, &trigger_names)
}

pub fn repair_triggers(database: &Database) -> Result<TriggerRepairReport, InstallError> {
    if database.layout().mode != InstallationMode::System {
        return Ok(TriggerRepairReport {
            repaired: Vec::new(),
            pending: Vec::new(),
            backend: "prefix-copy".to_owned(),
        });
    }

    let mut names = pending_triggers(database.layout())?
        .into_iter()
        .filter_map(|record| trigger_from_name(&record.name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        names = current_trigger_names(database)?;
    }
    let repaired = names
        .iter()
        .map(|trigger| trigger.as_str().to_owned())
        .collect();
    run_triggers(database, &names)?;

    Ok(TriggerRepairReport {
        repaired,
        pending: pending_triggers(database.layout())?,
        backend: "linux-copy".to_owned(),
    })
}

pub fn pending_triggers(layout: &StateLayout) -> Result<Vec<PendingTriggerRecord>, InstallError> {
    if layout.mode != InstallationMode::System {
        return Ok(Vec::new());
    }

    Ok(load_trigger_state(layout)?.pending)
}

fn run_triggers(database: &Database, trigger_names: &[SystemTrigger]) -> Result<(), InstallError> {
    if database.layout().mode != InstallationMode::System || trigger_names.is_empty() {
        return Ok(());
    }

    let mut state = load_trigger_state(database.layout())?;
    for trigger in trigger_names {
        match execute_trigger(database, *trigger) {
            Ok(output_path) => {
                state
                    .pending
                    .retain(|record| record.name != trigger.as_str());
                update_last_run(&mut state, *trigger, output_path);
            }
            Err(error) => {
                let pending = PendingTriggerRecord {
                    name: trigger.as_str().to_owned(),
                    reason: error.to_string(),
                };
                state.pending.retain(|record| record.name != pending.name);
                state.pending.push(pending);
            }
        }
    }
    state
        .pending
        .sort_by(|left, right| left.name.cmp(&right.name));
    state
        .last_run
        .sort_by(|left, right| left.name.cmp(&right.name));
    save_trigger_state(database.layout(), &state)
}

fn execute_trigger(database: &Database, trigger: SystemTrigger) -> Result<String, InstallError> {
    let paths = installed_paths(database)?;
    let output = match trigger {
        SystemTrigger::Ldconfig => json!({
            "trigger": trigger.as_str(),
            "libraries": filter_paths(&paths, |path| {
                (path.starts_with("/usr/lib/") || path.starts_with("/usr/lib64/"))
                    && path.rsplit_once('/').is_some_and(|(_, file_name)| file_name.contains(".so"))
            }),
        }),
        SystemTrigger::DesktopDb => json!({
            "trigger": trigger.as_str(),
            "desktop_entries": filter_paths(&paths, |path| {
                path.starts_with("/usr/share/applications/") && path.ends_with(".desktop")
            }),
        }),
        SystemTrigger::IconCache => json!({
            "trigger": trigger.as_str(),
            "icons": filter_paths(&paths, |path| path.starts_with("/usr/share/icons/")),
        }),
        SystemTrigger::FontCache => json!({
            "trigger": trigger.as_str(),
            "fonts": filter_paths(&paths, |path| path.starts_with("/usr/share/fonts/")),
        }),
        SystemTrigger::Depmod => json!({
            "trigger": trigger.as_str(),
            "modules": filter_paths(&paths, |path| path.starts_with("/usr/lib/modules/")),
        }),
        SystemTrigger::Initramfs => json!({
            "trigger": trigger.as_str(),
            "boot_inputs": filter_paths(&paths, |path| {
                path.starts_with("/boot/") || path.starts_with("/usr/lib/modules/")
            }),
        }),
    };

    let output_path = trigger_output_path(database.layout(), trigger);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&output)?)?;

    Ok(output_path.display().to_string())
}

fn installed_paths(database: &Database) -> Result<Vec<String>, InstallError> {
    let mut paths = BTreeSet::new();
    for package in database.list_installed_packages()? {
        for record in database.package_files(&package.pkgname)? {
            paths.insert(record.path);
        }
    }

    Ok(paths.into_iter().collect())
}

fn current_trigger_names(database: &Database) -> Result<Vec<SystemTrigger>, InstallError> {
    let paths = installed_paths(database)?;
    Ok(detect_trigger_names(paths.iter().map(String::as_str)))
}

fn filter_paths<F>(paths: &[String], predicate: F) -> Vec<String>
where
    F: Fn(&str) -> bool,
{
    paths
        .iter()
        .filter(|path| predicate(path))
        .cloned()
        .collect::<Vec<_>>()
}

fn update_last_run(state: &mut TriggerState, trigger: SystemTrigger, output_path: String) {
    state
        .last_run
        .retain(|record| record.name != trigger.as_str());
    state.last_run.push(TriggerRunRecord {
        name: trigger.as_str().to_owned(),
        output_path,
    });
}

fn load_trigger_state(layout: &StateLayout) -> Result<TriggerState, InstallError> {
    let path = trigger_state_path(layout);
    if !path.exists() {
        return Ok(TriggerState::default());
    }

    Ok(serde_json::from_slice::<TriggerState>(&fs::read(path)?)?)
}

fn save_trigger_state(layout: &StateLayout, state: &TriggerState) -> Result<(), InstallError> {
    let path = trigger_state_path(layout);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;

    Ok(())
}

fn trigger_state_path(layout: &StateLayout) -> PathBuf {
    layout
        .state_dir
        .join("system-backend")
        .join("triggers.json")
}

fn trigger_output_path(layout: &StateLayout, trigger: SystemTrigger) -> PathBuf {
    layout
        .state_dir
        .join("system-backend")
        .join("triggers")
        .join(format!("{}.json", trigger.as_str()))
}

fn trigger_from_name(name: &str) -> Option<SystemTrigger> {
    match name {
        "ldconfig" => Some(SystemTrigger::Ldconfig),
        "desktop_db" => Some(SystemTrigger::DesktopDb),
        "icon_cache" => Some(SystemTrigger::IconCache),
        "font_cache" => Some(SystemTrigger::FontCache),
        "depmod" => Some(SystemTrigger::Depmod),
        "initramfs" => Some(SystemTrigger::Initramfs),
        _ => None,
    }
}
