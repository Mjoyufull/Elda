use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use elda_build::BuiltPackage;
use elda_db::{Database, InstallationMode, StateLayout};
use elda_linux::{SystemTrigger, activation_backend_for_system_mode, detect_trigger_names};

use crate::InstallError;

use super::trigger_model::{
    BootStatusReport, PendingTriggerRecord, TriggerRepairReport, TriggerRunRecord,
    TriggerStateReport,
};

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

pub fn run_named_trigger(
    database: &Database,
    name: &str,
) -> Result<TriggerRepairReport, InstallError> {
    let trigger = trigger_from_name(name)
        .ok_or_else(|| InstallError::Unsupported(format!("unknown system trigger `{name}`")))?;
    if database.layout().mode != InstallationMode::System {
        return Err(InstallError::Unsupported(
            "trigger run requires system activation mode (`-S`)".to_owned(),
        ));
    }

    run_triggers(database, &[trigger])?;

    Ok(TriggerRepairReport {
        repaired: vec![trigger.as_str().to_owned()],
        pending: pending_triggers(database.layout())?,
        backend: activation_backend_for_system_mode(true).name().to_owned(),
        boot_status: Some(load_boot_status(database.layout())?),
    })
}

pub fn repair_triggers(database: &Database) -> Result<TriggerRepairReport, InstallError> {
    if database.layout().mode != InstallationMode::System {
        return Ok(TriggerRepairReport {
            repaired: Vec::new(),
            pending: Vec::new(),
            backend: activation_backend_for_system_mode(false).name().to_owned(),
            boot_status: None,
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
        backend: activation_backend_for_system_mode(true).name().to_owned(),
        boot_status: Some(load_boot_status(database.layout())?),
    })
}

pub fn pending_triggers(layout: &StateLayout) -> Result<Vec<PendingTriggerRecord>, InstallError> {
    if layout.mode != InstallationMode::System {
        return Ok(Vec::new());
    }

    Ok(load_trigger_state(layout)?.pending)
}

pub fn load_boot_status(layout: &StateLayout) -> Result<BootStatusReport, InstallError> {
    if layout.mode != InstallationMode::System {
        return Ok(BootStatusReport::default());
    }

    let path = boot_state_path(layout);
    let mut state = if path.exists() {
        serde_json::from_slice::<BootStatusReport>(&fs::read(path)?)?
    } else {
        BootStatusReport::default()
    };
    let trigger_state = load_trigger_state(layout)?;
    state.pending_triggers = trigger_state
        .pending
        .iter()
        .filter(|record| {
            record.boot_path
                || trigger_from_name(&record.name).is_some_and(|trigger| trigger.is_boot_trigger())
        })
        .cloned()
        .collect();
    state.last_run = trigger_state
        .last_run
        .iter()
        .filter(|record| {
            trigger_from_name(&record.name).is_some_and(|trigger| trigger.is_boot_trigger())
        })
        .cloned()
        .collect();

    Ok(state)
}

fn run_triggers(database: &Database, trigger_names: &[SystemTrigger]) -> Result<(), InstallError> {
    if database.layout().mode != InstallationMode::System || trigger_names.is_empty() {
        return Ok(());
    }

    let paths = installed_paths(database)?;
    let mut state = load_trigger_state(database.layout())?;
    for trigger in trigger_names {
        match execute_trigger(database.layout(), *trigger, &paths) {
            Ok(output_path) => {
                state
                    .pending
                    .retain(|record| record.name != trigger.as_str());
                update_last_run(&mut state, *trigger, output_path);
            }
            Err(error) => {
                let pending = pending_trigger_record(*trigger, error.to_string());
                state.pending.retain(|record| record.name != pending.name);
                state.pending.push(pending);
                finalize_trigger_state(database.layout(), &state, &paths)?;
                if trigger.is_critical() {
                    return Err(error);
                }
            }
        }
    }
    finalize_trigger_state(database.layout(), &state, &paths)
}

fn finalize_trigger_state(
    layout: &StateLayout,
    state: &TriggerStateReport,
    paths: &[String],
) -> Result<(), InstallError> {
    let mut state = state.clone();
    state
        .pending
        .sort_by(|left, right| left.name.cmp(&right.name));
    state
        .last_run
        .sort_by(|left, right| left.name.cmp(&right.name));
    save_trigger_state(layout, &state)?;
    save_boot_status(layout, &boot_status_from_state(&state, paths))
}

fn execute_trigger(
    layout: &StateLayout,
    trigger: SystemTrigger,
    paths: &[String],
) -> Result<String, InstallError> {
    let output = match trigger {
        SystemTrigger::Ldconfig => json!({
            "trigger": trigger.as_str(),
            "libraries": filter_paths(paths, |path| {
                (path.starts_with("/usr/lib/") || path.starts_with("/usr/lib64/"))
                    && path.rsplit_once('/').is_some_and(|(_, file_name)| file_name.contains(".so"))
            }),
        }),
        SystemTrigger::DesktopDb => json!({
            "trigger": trigger.as_str(),
            "desktop_entries": filter_paths(paths, |path| {
                path.starts_with("/usr/share/applications/") && path.ends_with(".desktop")
            }),
        }),
        SystemTrigger::IconCache => json!({
            "trigger": trigger.as_str(),
            "icons": filter_paths(paths, |path| path.starts_with("/usr/share/icons/")),
        }),
        SystemTrigger::FontCache => json!({
            "trigger": trigger.as_str(),
            "fonts": filter_paths(paths, |path| path.starts_with("/usr/share/fonts/")),
        }),
        SystemTrigger::Depmod => json!({
            "trigger": trigger.as_str(),
            "modules": filter_paths(paths, |path| path.starts_with("/usr/lib/modules/")),
        }),
        SystemTrigger::Initramfs => json!({
            "trigger": trigger.as_str(),
            "boot_inputs": filter_paths(paths, |path| {
                path.starts_with("/boot/") || path.starts_with("/usr/lib/modules/")
            }),
        }),
    };

    let output_path = trigger_output_path(layout, trigger);
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

fn update_last_run(state: &mut TriggerStateReport, trigger: SystemTrigger, output_path: String) {
    state
        .last_run
        .retain(|record| record.name != trigger.as_str());
    state.last_run.push(TriggerRunRecord {
        name: trigger.as_str().to_owned(),
        output_path,
    });
}

pub(super) fn load_trigger_state(layout: &StateLayout) -> Result<TriggerStateReport, InstallError> {
    let path = trigger_state_path(layout);
    if !path.exists() {
        return Ok(TriggerStateReport::default());
    }

    Ok(serde_json::from_slice::<TriggerStateReport>(&fs::read(
        path,
    )?)?)
}

fn save_trigger_state(
    layout: &StateLayout,
    state: &TriggerStateReport,
) -> Result<(), InstallError> {
    let path = trigger_state_path(layout);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(state)?)?;

    Ok(())
}

fn save_boot_status(layout: &StateLayout, state: &BootStatusReport) -> Result<(), InstallError> {
    let path = boot_state_path(layout);
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

fn boot_state_path(layout: &StateLayout) -> PathBuf {
    layout.state_dir.join("system-backend").join("boot.json")
}

pub(super) fn trigger_output_path(layout: &StateLayout, trigger: SystemTrigger) -> PathBuf {
    layout
        .state_dir
        .join("system-backend")
        .join("triggers")
        .join(format!("{}.json", trigger.as_str()))
}

fn boot_status_from_state(state: &TriggerStateReport, paths: &[String]) -> BootStatusReport {
    BootStatusReport {
        managed_inputs: filter_paths(paths, |path| {
            path.starts_with("/boot/") || path.starts_with("/usr/lib/modules/")
        }),
        pending_triggers: state
            .pending
            .iter()
            .filter(|record| record.boot_path)
            .cloned()
            .collect(),
        last_run: state
            .last_run
            .iter()
            .filter(|record| {
                trigger_from_name(&record.name).is_some_and(SystemTrigger::is_boot_trigger)
            })
            .cloned()
            .collect(),
    }
}

fn pending_trigger_record(trigger: SystemTrigger, reason: String) -> PendingTriggerRecord {
    PendingTriggerRecord {
        name: trigger.as_str().to_owned(),
        reason,
        boot_path: trigger.is_boot_trigger(),
        critical: trigger.is_critical(),
    }
}

pub(super) fn trigger_from_name(name: &str) -> Option<SystemTrigger> {
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
