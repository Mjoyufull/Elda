use std::collections::BTreeSet;
use std::io::ErrorKind;

use serde::Serialize;
use serde_json::json;

use crate::app::{AppContext, PlannedInstallAction};
use crate::error::CoreError;

#[derive(Debug, Clone, Default, Serialize)]
pub(crate) struct ProfilePolicyResolution {
    pub(crate) native_arch: Option<String>,
    pub(crate) foreign_arches: Vec<String>,
    pub(crate) init: Option<String>,
    pub(crate) declared_by: Vec<ProfilePolicySource>,
    pub(crate) unresolved_profiles: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProfilePolicySource {
    pub(crate) profile: String,
    pub(crate) native_arch: Option<String>,
    pub(crate) foreign_arches: Vec<String>,
    pub(crate) init: Option<String>,
    pub(crate) source: &'static str,
}

impl AppContext {
    pub(crate) fn resolve_requested_profile_policy(
        &self,
        install_plan: &[PlannedInstallAction],
    ) -> Result<ProfilePolicyResolution, CoreError> {
        let mut resolution = ProfilePolicyResolution::default();

        for action in install_plan
            .iter()
            .filter(|action| action.install_reason == "explicit")
        {
            let Some(policy) = &action.resolved.recipe.package.profile else {
                continue;
            };
            let source = ProfilePolicySource {
                profile: action.package_name.clone(),
                native_arch: policy.native_arch.clone(),
                foreign_arches: policy.foreign_arches.clone(),
                init: policy.init.clone(),
                source: "recipe",
            };
            merge_profile_policy_source(&mut resolution, source)?;
        }

        Ok(resolution)
    }

    pub(crate) fn resolve_local_profile_policy(
        &self,
        active_profiles: &[String],
    ) -> Result<ProfilePolicyResolution, CoreError> {
        let mut resolution = ProfilePolicyResolution::default();

        for profile_name in active_profiles {
            match elda_recipe::load_recipe(&self.database.layout().recipes_dir, profile_name) {
                Ok(recipe) => {
                    let Some(policy) = &recipe.package.profile else {
                        continue;
                    };
                    let source = ProfilePolicySource {
                        profile: profile_name.clone(),
                        native_arch: policy.native_arch.clone(),
                        foreign_arches: policy.foreign_arches.clone(),
                        init: policy.init.clone(),
                        source: "local-recipe",
                    };
                    merge_profile_policy_source(&mut resolution, source)?;
                }
                Err(elda_recipe::RecipeError::Io(error)) if error.kind() == ErrorKind::NotFound => {
                    resolution.unresolved_profiles.push(profile_name.clone());
                }
                Err(error) => return Err(CoreError::Recipe(error)),
            }
        }

        resolution.unresolved_profiles =
            dedupe_preserve_order(resolution.unresolved_profiles.clone());

        Ok(resolution)
    }
}

pub(crate) fn profile_policy_json(resolution: &ProfilePolicyResolution) -> serde_json::Value {
    json!({
        "resolved": {
            "native_arch": resolution.native_arch,
            "foreign_arches": resolution.foreign_arches,
            "init": resolution.init,
        },
        "declared_by": resolution.declared_by,
        "unresolved_profiles": resolution.unresolved_profiles,
    })
}

fn merge_profile_policy_source(
    resolution: &mut ProfilePolicyResolution,
    source: ProfilePolicySource,
) -> Result<(), CoreError> {
    if let Some(native_arch) = &source.native_arch {
        match &resolution.native_arch {
            Some(existing) if existing != native_arch => {
                return Err(CoreError::Operator(format!(
                    "profile policy conflict: `{}` declares native_arch `{native_arch}`, but another selected profile already declared `{existing}`",
                    source.profile,
                )));
            }
            None => resolution.native_arch = Some(native_arch.clone()),
            Some(_) => {}
        }
    }

    if let Some(init) = &source.init {
        match &resolution.init {
            Some(existing) if existing != init => {
                return Err(CoreError::Operator(format!(
                    "profile policy conflict: `{}` declares init-provider `{init}`, but another selected profile already declared `{existing}`",
                    source.profile,
                )));
            }
            None => resolution.init = Some(init.clone()),
            Some(_) => {}
        }
    }

    let mut seen_arches = resolution
        .foreign_arches
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    for arch in &source.foreign_arches {
        if seen_arches.insert(arch.clone()) {
            resolution.foreign_arches.push(arch.clone());
        }
    }

    resolution.declared_by.push(source);

    Ok(())
}

fn dedupe_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }

    deduped
}
