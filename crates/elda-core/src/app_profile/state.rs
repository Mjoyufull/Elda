use std::fs;
use std::path::PathBuf;

use crate::app::{AppContext, DesiredStateProfile, ResolvedProfileState};
use crate::config::default_native_arch;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::{empty_to, profile_details_json};

impl AppContext {
    pub(crate) fn handle_profile_show(
        &self,
        request: CommandRequest,
    ) -> Result<CommandReport, CoreError> {
        self.database.bootstrap()?;
        let profile = self.resolve_profile_state()?;
        let declared_policy = self.resolve_local_profile_policy(&profile.active_profiles)?;
        let runtime_view = self.profile_runtime_view(&profile)?;

        Ok(CommandReport {
            area: "profile",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: "reported the current machine profile state.".to_owned(),
            details: Some(profile_details_json(
                &profile,
                &declared_policy,
                &runtime_view,
            )),
        })
    }

    pub(crate) fn profile_state_path(&self) -> PathBuf {
        self.database.layout().db_dir.join("profile-state.json")
    }

    pub(crate) fn load_profile_state(&self) -> Result<Option<DesiredStateProfile>, CoreError> {
        let path = self.profile_state_path();
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let profile = serde_json::from_str::<DesiredStateProfile>(&content)?;

        Ok(Some(profile))
    }

    pub(crate) fn write_profile_state(
        &self,
        profile: &DesiredStateProfile,
    ) -> Result<(), CoreError> {
        let path = self.profile_state_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(profile)?)?;

        Ok(())
    }

    pub(crate) fn profile_state_base(
        &self,
        resolved: &ResolvedProfileState,
    ) -> Result<String, CoreError> {
        if let Some(profile) = self.load_profile_state()?
            && !profile.base.trim().is_empty()
        {
            return Ok(profile.base);
        }
        if !self.config.profile.base.trim().is_empty() {
            return Ok(self.config.profile.base.clone());
        }

        Ok(resolved
            .active_profiles
            .first()
            .cloned()
            .unwrap_or_default())
    }

    pub(crate) fn resolve_profile_state(&self) -> Result<ResolvedProfileState, CoreError> {
        if let Some(profile) = self.load_profile_state()? {
            return Ok(ResolvedProfileState {
                active_profiles: profile.active_profiles,
                native_arch: empty_to(profile.native_arch, default_native_arch()),
                foreign_arches: profile.foreign_arches,
                init: profile.init,
            });
        }

        let installed_profiles = self
            .database
            .list_installed_packages()?
            .into_iter()
            .filter(|package| package.package_kind == "profile")
            .map(|package| package.pkgname)
            .collect::<Vec<_>>();

        let mut active_profiles = installed_profiles;
        if active_profiles.is_empty() && !self.config.profile.base.is_empty() {
            active_profiles.push(self.config.profile.base.clone());
        }

        Ok(ResolvedProfileState {
            active_profiles,
            native_arch: empty_to(
                self.config.profile.native_arch.clone(),
                default_native_arch(),
            ),
            foreign_arches: self.config.profile.foreign_arches.clone(),
            init: self.config.profile.init.clone(),
        })
    }
}

impl ResolvedProfileState {
    #[must_use]
    pub(crate) fn to_desired_profile(&self, base: String) -> DesiredStateProfile {
        DesiredStateProfile {
            active_profiles: self.active_profiles.clone(),
            base,
            native_arch: self.native_arch.clone(),
            foreign_arches: self.foreign_arches.clone(),
            init: self.init.clone(),
        }
    }
}
