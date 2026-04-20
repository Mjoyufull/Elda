use serde::Serialize;

use crate::app::{AppContext, ResolvedProfileState};
use crate::config::default_native_arch;
use crate::error::CoreError;
use elda_db::InstallationMode;

const PREFIX_PROFILE_BACKEND: &str = "prefix-copy";
const SYSTEM_PROFILE_BACKEND: &str = "linux-copy";
const DEFERRED_REASON: &str =
    "the current backend does not reconcile provider-scoped system changes yet";
const SYSTEM_PROVIDER_REASON: &str =
    "provider-scoped asset reconciliation can be applied on the current backend via `fix-triggers`";
const SYSTEM_INIT_REASON: &str =
    "the current backend can reconcile the active init-provider asset set directly";

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfileRuntimeView {
    pub(crate) provider_families: ProviderFamilies,
    pub(crate) pending_handler_transitions: Vec<PendingSystemChange>,
    pub(crate) required_activation_class: &'static str,
    pub(crate) backend: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderFamilies {
    pub(crate) init: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PendingSystemChange {
    pub(crate) kind: &'static str,
    pub(crate) summary: String,
    pub(crate) activation_class: &'static str,
    pub(crate) backend: &'static str,
    pub(crate) supported: bool,
    pub(crate) reason: &'static str,
    pub(crate) current: SystemChangeState,
    pub(crate) desired: SystemChangeState,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) added_foreign_arches: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) removed_foreign_arches: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct SystemChangeState {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) active_profiles: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub(crate) init: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) foreign_arches: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ActivationClass {
    None,
    Live,
    RebootRequired,
}

impl ActivationClass {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Live => "live",
            Self::RebootRequired => "reboot-required",
        }
    }
}

impl AppContext {
    pub(crate) fn profile_runtime_view(
        &self,
        desired: &ResolvedProfileState,
    ) -> Result<ProfileRuntimeView, CoreError> {
        let applied = self.resolve_applied_profile_state()?;
        Ok(build_profile_runtime_view(
            &applied,
            desired,
            self.database.layout().mode == InstallationMode::System,
        ))
    }

    fn resolve_applied_profile_state(&self) -> Result<ResolvedProfileState, CoreError> {
        let active_profiles = self
            .database
            .list_installed_packages()?
            .into_iter()
            .filter(|package| package.package_kind == "profile")
            .map(|package| package.pkgname)
            .collect::<Vec<_>>();

        let system_backend = self.database.layout().mode == InstallationMode::System;
        let applied_init = if system_backend {
            elda_install::load_applied_profile_init(self.database.layout())?
        } else {
            String::new()
        };

        Ok(ResolvedProfileState {
            active_profiles,
            native_arch: default_native_arch(),
            foreign_arches: Vec::new(),
            init: applied_init,
        })
    }
}

fn build_profile_runtime_view(
    applied: &ResolvedProfileState,
    desired: &ResolvedProfileState,
    system_backend: bool,
) -> ProfileRuntimeView {
    let mut pending = Vec::new();
    let mut strongest = ActivationClass::None;
    let backend = if system_backend {
        SYSTEM_PROFILE_BACKEND
    } else {
        PREFIX_PROFILE_BACKEND
    };

    if applied.active_profiles != desired.active_profiles {
        push_transition(
            &mut pending,
            &mut strongest,
            PendingSystemChange {
                kind: "profile-provider-reconciliation",
                summary: "provider-family reconciliation is pending for the requested profile set."
                    .to_owned(),
                activation_class: ActivationClass::Live.as_str(),
                backend,
                supported: system_backend,
                reason: if system_backend {
                    SYSTEM_PROVIDER_REASON
                } else {
                    DEFERRED_REASON
                },
                current: SystemChangeState {
                    active_profiles: applied.active_profiles.clone(),
                    ..SystemChangeState::default()
                },
                desired: SystemChangeState {
                    active_profiles: desired.active_profiles.clone(),
                    ..SystemChangeState::default()
                },
                added_foreign_arches: Vec::new(),
                removed_foreign_arches: Vec::new(),
            },
            ActivationClass::Live,
        );
    }

    if applied.init != desired.init {
        let init_transition_supported = system_backend;
        let activation_class = if init_transition_supported {
            ActivationClass::Live
        } else {
            ActivationClass::RebootRequired
        };
        push_transition(
            &mut pending,
            &mut strongest,
            PendingSystemChange {
                kind: "init-provider-transition",
                summary: format!(
                    "init-provider transition from `{}` to `{}` is pending.",
                    empty_to_unset(&applied.init),
                    empty_to_unset(&desired.init),
                ),
                activation_class: activation_class.as_str(),
                backend,
                supported: init_transition_supported,
                reason: if init_transition_supported {
                    SYSTEM_INIT_REASON
                } else {
                    DEFERRED_REASON
                },
                current: SystemChangeState {
                    init: applied.init.clone(),
                    ..SystemChangeState::default()
                },
                desired: SystemChangeState {
                    init: desired.init.clone(),
                    ..SystemChangeState::default()
                },
                added_foreign_arches: Vec::new(),
                removed_foreign_arches: Vec::new(),
            },
            activation_class,
        );
    }

    let (added_foreign_arches, removed_foreign_arches) =
        diff_foreign_arches(&applied.foreign_arches, &desired.foreign_arches);
    if !added_foreign_arches.is_empty() || !removed_foreign_arches.is_empty() {
        push_transition(
            &mut pending,
            &mut strongest,
            PendingSystemChange {
                kind: "multilib-policy-transition",
                summary: "foreign-architecture policy reconciliation is pending.".to_owned(),
                activation_class: ActivationClass::Live.as_str(),
                backend,
                supported: false,
                reason: DEFERRED_REASON,
                current: SystemChangeState {
                    foreign_arches: applied.foreign_arches.clone(),
                    ..SystemChangeState::default()
                },
                desired: SystemChangeState {
                    foreign_arches: desired.foreign_arches.clone(),
                    ..SystemChangeState::default()
                },
                added_foreign_arches,
                removed_foreign_arches,
            },
            ActivationClass::Live,
        );
    }

    ProfileRuntimeView {
        provider_families: ProviderFamilies {
            init: desired.init.clone(),
        },
        pending_handler_transitions: pending,
        required_activation_class: strongest.as_str(),
        backend,
    }
}

fn push_transition(
    pending: &mut Vec<PendingSystemChange>,
    strongest: &mut ActivationClass,
    change: PendingSystemChange,
    activation_class: ActivationClass,
) {
    *strongest = (*strongest).max(activation_class);
    pending.push(change);
}

fn diff_foreign_arches(current: &[String], desired: &[String]) -> (Vec<String>, Vec<String>) {
    let added = desired
        .iter()
        .filter(|arch| !current.contains(*arch))
        .cloned()
        .collect::<Vec<_>>();
    let removed = current
        .iter()
        .filter(|arch| !desired.contains(*arch))
        .cloned()
        .collect::<Vec<_>>();

    (added, removed)
}

fn empty_to_unset(value: &str) -> &str {
    if value.trim().is_empty() {
        "unset"
    } else {
        value
    }
}
