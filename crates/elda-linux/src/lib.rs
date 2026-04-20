#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use elda_types::CrateBoundary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationBackend {
    PrefixCopy,
    LinuxCopy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationBackendCapabilities {
    pub live_activation: bool,
    pub reboot_only: bool,
    pub boot_integrated: bool,
    pub archives_states: bool,
}

impl ActivationBackend {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PrefixCopy => "prefix-copy",
            Self::LinuxCopy => "linux-copy",
        }
    }

    #[must_use]
    pub const fn state_prefix(self) -> &'static str {
        match self {
            Self::PrefixCopy => "prefix",
            Self::LinuxCopy => "system",
        }
    }

    #[must_use]
    pub const fn capabilities(self) -> ActivationBackendCapabilities {
        match self {
            Self::PrefixCopy => ActivationBackendCapabilities {
                live_activation: true,
                reboot_only: false,
                boot_integrated: false,
                archives_states: false,
            },
            Self::LinuxCopy => ActivationBackendCapabilities {
                live_activation: true,
                reboot_only: false,
                boot_integrated: true,
                archives_states: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemTrigger {
    Ldconfig,
    DesktopDb,
    IconCache,
    FontCache,
    Depmod,
    Initramfs,
}

impl SystemTrigger {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ldconfig => "ldconfig",
            Self::DesktopDb => "desktop_db",
            Self::IconCache => "icon_cache",
            Self::FontCache => "font_cache",
            Self::Depmod => "depmod",
            Self::Initramfs => "initramfs",
        }
    }

    #[must_use]
    pub const fn is_boot_trigger(self) -> bool {
        matches!(self, Self::Depmod | Self::Initramfs)
    }

    #[must_use]
    pub const fn is_critical(self) -> bool {
        matches!(self, Self::Initramfs)
    }
}

#[must_use]
pub fn detect_trigger_names<'a>(paths: impl IntoIterator<Item = &'a str>) -> Vec<SystemTrigger> {
    let mut detected = BTreeSet::new();

    for path in paths {
        if is_shared_library_path(path) {
            detected.insert(SystemTrigger::Ldconfig);
        }
        if path.starts_with("/usr/share/applications/") && path.ends_with(".desktop") {
            detected.insert(SystemTrigger::DesktopDb);
        }
        if path.starts_with("/usr/share/icons/") {
            detected.insert(SystemTrigger::IconCache);
        }
        if path.starts_with("/usr/share/fonts/") {
            detected.insert(SystemTrigger::FontCache);
        }
        if path.starts_with("/usr/lib/modules/") {
            detected.insert(SystemTrigger::Depmod);
            detected.insert(SystemTrigger::Initramfs);
        }
        if path.starts_with("/boot/") {
            detected.insert(SystemTrigger::Initramfs);
        }
    }

    detected.into_iter().collect()
}

#[must_use]
pub const fn activation_backend_for_system_mode(system_mode: bool) -> ActivationBackend {
    if system_mode {
        ActivationBackend::LinuxCopy
    } else {
        ActivationBackend::PrefixCopy
    }
}

fn is_shared_library_path(path: &str) -> bool {
    (path.starts_with("/usr/lib/") || path.starts_with("/usr/lib64/"))
        && path
            .rsplit_once('/')
            .is_some_and(|(_, file_name)| file_name.contains(".so"))
}

pub const BOUNDARY: CrateBoundary = CrateBoundary::new(
    "elda-linux",
    "Linux-only activation, multilib, and namespace implementations.",
);
