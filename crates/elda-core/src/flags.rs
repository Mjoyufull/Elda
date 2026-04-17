use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::app::{AppContext, ParsedInstallRequest};
use crate::error::CoreError;
use elda_recipe::{LuaValue, PackageDefinition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedFlagState {
    pub(crate) active_profiles: Vec<String>,
    pub(crate) allowed_flags: Vec<String>,
    pub(crate) default_flags: BTreeMap<String, bool>,
    pub(crate) global_flags: BTreeMap<String, bool>,
    pub(crate) profile_flags: BTreeMap<String, bool>,
    pub(crate) package_flags: BTreeMap<String, bool>,
    pub(crate) cli_flags: BTreeMap<String, bool>,
    pub(crate) effective_flags: BTreeMap<String, bool>,
    pub(crate) variant_id: String,
    pub(crate) customized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageFlagPolicy {
    allowed: BTreeSet<String>,
    defaults: BTreeMap<String, bool>,
    implies: BTreeMap<String, Vec<String>>,
    conflicts: BTreeMap<String, Vec<String>>,
}

impl AppContext {
    pub(crate) fn resolve_flag_state(
        &self,
        package: &PackageDefinition,
        request: &ParsedInstallRequest,
    ) -> Result<ResolvedFlagState, CoreError> {
        let policy = PackageFlagPolicy::from_package(package)?;
        let profile = self.resolve_profile_state()?;
        let global_flags = self.config.flags.global.clone();
        let profile_flags =
            merge_profile_flags(&self.config.flags.profile, &profile.active_profiles);
        let package_flags = self
            .config
            .flags
            .package
            .get(&package.name)
            .cloned()
            .unwrap_or_default();
        let cli_flags = request.cli_flag_overrides.clone();

        validate_flag_names(&policy.allowed, "global flag", &global_flags)?;
        validate_flag_names(&policy.allowed, "profile flag", &profile_flags)?;
        validate_flag_names(&policy.allowed, "package flag", &package_flags)?;
        validate_flag_names(&policy.allowed, "CLI flag", &cli_flags)?;

        let mut effective_flags = normalized_flag_map(&policy.allowed, &policy.defaults);
        apply_flag_overrides(&mut effective_flags, &global_flags);
        apply_flag_overrides(&mut effective_flags, &profile_flags);
        apply_flag_overrides(&mut effective_flags, &package_flags);
        apply_flag_overrides(&mut effective_flags, &cli_flags);
        apply_implied_flags(&policy, &mut effective_flags)?;
        validate_flag_conflicts(package, &policy, &effective_flags)?;

        let default_flags = normalized_flag_map(&policy.allowed, &policy.defaults);
        let variant_id = variant_id_for_flags(&effective_flags);

        Ok(ResolvedFlagState {
            active_profiles: profile.active_profiles,
            allowed_flags: policy.allowed.iter().cloned().collect(),
            default_flags: default_flags.clone(),
            global_flags,
            profile_flags,
            package_flags,
            cli_flags,
            effective_flags: effective_flags.clone(),
            variant_id,
            customized: effective_flags != default_flags,
        })
    }
}

pub(crate) fn parse_cli_flag_list(value: &str) -> Result<BTreeMap<String, bool>, CoreError> {
    let mut flags = BTreeMap::new();

    for raw_token in value.split(',') {
        let token = raw_token.trim();
        if token.is_empty() {
            continue;
        }

        let (enabled, name) = if let Some(name) = token.strip_prefix('+') {
            (true, name)
        } else if let Some(name) = token.strip_prefix('-') {
            (false, name)
        } else {
            return Err(CoreError::Operator(format!(
                "invalid `--use` token `{token}`; expected `+flag` or `-flag`"
            )));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::Operator(
                "invalid empty flag name in `--use`".to_owned(),
            ));
        }
        flags.insert(name.to_owned(), enabled);
    }

    if flags.is_empty() {
        return Err(CoreError::Operator(
            "`--use` requires at least one `+flag` or `-flag` entry".to_owned(),
        ));
    }

    Ok(flags)
}

impl PackageFlagPolicy {
    fn from_package(package: &PackageDefinition) -> Result<Self, CoreError> {
        let defaults = parse_bool_table(package.flags_default.as_ref(), "flags_default")?;
        let allowed = parse_allowed_flags(package, &defaults)?;
        let implies = parse_string_list_table(package.flags_implies.as_ref(), "flags_implies")?;
        let conflicts =
            parse_string_list_table(package.flags_conflicts.as_ref(), "flags_conflicts")?;

        validate_referenced_flags(&allowed, "flags_implies", &implies)?;
        validate_referenced_flags(&allowed, "flags_conflicts", &conflicts)?;

        Ok(Self {
            allowed,
            defaults,
            implies,
            conflicts,
        })
    }
}

fn parse_allowed_flags(
    package: &PackageDefinition,
    defaults: &BTreeMap<String, bool>,
) -> Result<BTreeSet<String>, CoreError> {
    let allowed_table = parse_bool_table(package.flags_allowed.as_ref(), "flags_allowed")?;
    let implies = parse_string_list_table(package.flags_implies.as_ref(), "flags_implies")?;
    let conflicts = parse_string_list_table(package.flags_conflicts.as_ref(), "flags_conflicts")?;
    let mut allowed = allowed_table.into_keys().collect::<BTreeSet<_>>();
    allowed.extend(defaults.keys().cloned());
    for (flag, values) in implies.iter().chain(conflicts.iter()) {
        allowed.insert(flag.clone());
        allowed.extend(values.iter().cloned());
    }

    Ok(allowed)
}

fn parse_bool_table(
    value: Option<&LuaValue>,
    field: &str,
) -> Result<BTreeMap<String, bool>, CoreError> {
    let Some(LuaValue::Table(table)) = value else {
        return Ok(BTreeMap::new());
    };

    let mut parsed = BTreeMap::new();
    for (name, value) in table {
        let LuaValue::Boolean(enabled) = value else {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("{field}.{name} must be a boolean"),
            )));
        };
        parsed.insert(name.clone(), *enabled);
    }

    Ok(parsed)
}

fn parse_string_list_table(
    value: Option<&LuaValue>,
    field: &str,
) -> Result<BTreeMap<String, Vec<String>>, CoreError> {
    let Some(LuaValue::Table(table)) = value else {
        return Ok(BTreeMap::new());
    };

    let mut parsed = BTreeMap::new();
    for (name, value) in table {
        let LuaValue::Array(entries) = value else {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("{field}.{name} must be an array of flag names"),
            )));
        };
        let mut values = Vec::with_capacity(entries.len());
        for entry in entries {
            let LuaValue::String(flag) = entry else {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!("{field}.{name} must contain only string flag names"),
                )));
            };
            if flag.trim().is_empty() {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!("{field}.{name} must not contain empty flag names"),
                )));
            }
            values.push(flag.clone());
        }
        parsed.insert(name.clone(), values);
    }

    Ok(parsed)
}

fn validate_referenced_flags(
    allowed: &BTreeSet<String>,
    field: &str,
    table: &BTreeMap<String, Vec<String>>,
) -> Result<(), CoreError> {
    for (flag, values) in table {
        if !allowed.contains(flag) {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("{field}.{flag} references an undeclared flag"),
            )));
        }
        for value in values {
            if !allowed.contains(value) {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!("{field}.{flag} references undeclared flag `{value}`"),
                )));
            }
        }
    }

    Ok(())
}

fn validate_flag_names(
    allowed: &BTreeSet<String>,
    layer_name: &str,
    flags: &BTreeMap<String, bool>,
) -> Result<(), CoreError> {
    for flag in flags.keys() {
        if !allowed.contains(flag) {
            return Err(CoreError::Operator(format!(
                "{layer_name} `{flag}` is not declared by this package"
            )));
        }
    }

    Ok(())
}

fn merge_profile_flags(
    profiles: &BTreeMap<String, BTreeMap<String, bool>>,
    active_profiles: &[String],
) -> BTreeMap<String, bool> {
    let mut merged = BTreeMap::new();

    for profile in active_profiles {
        if let Some(flags) = profiles.get(profile) {
            apply_flag_overrides(&mut merged, flags);
        }
    }

    merged
}

fn normalized_flag_map(
    allowed: &BTreeSet<String>,
    defaults: &BTreeMap<String, bool>,
) -> BTreeMap<String, bool> {
    allowed
        .iter()
        .map(|flag| (flag.clone(), defaults.get(flag).copied().unwrap_or(false)))
        .collect()
}

fn apply_flag_overrides(
    effective: &mut BTreeMap<String, bool>,
    overrides: &BTreeMap<String, bool>,
) {
    for (flag, enabled) in overrides {
        effective.insert(flag.clone(), *enabled);
    }
}

fn apply_implied_flags(
    policy: &PackageFlagPolicy,
    effective: &mut BTreeMap<String, bool>,
) -> Result<(), CoreError> {
    loop {
        let mut changed = false;
        for (flag, implies) in &policy.implies {
            if !effective.get(flag).copied().unwrap_or(false) {
                continue;
            }
            for implied in implies {
                let Some(current) = effective.get_mut(implied) else {
                    return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                        format!("flag implication references undeclared flag `{implied}`"),
                    )));
                };
                if !*current {
                    *current = true;
                    changed = true;
                }
            }
        }
        if !changed {
            return Ok(());
        }
    }
}

fn validate_flag_conflicts(
    package: &PackageDefinition,
    policy: &PackageFlagPolicy,
    effective: &BTreeMap<String, bool>,
) -> Result<(), CoreError> {
    for (flag, conflicts) in &policy.conflicts {
        if !effective.get(flag).copied().unwrap_or(false) {
            continue;
        }
        for conflict in conflicts {
            if effective.get(conflict).copied().unwrap_or(false) {
                return Err(CoreError::Operator(format!(
                    "package `{}` resolves conflicting flags `{flag}` and `{conflict}`",
                    package.name
                )));
            }
        }
    }

    Ok(())
}

fn variant_id_for_flags(effective: &BTreeMap<String, bool>) -> String {
    if effective.is_empty() {
        return "default".to_owned();
    }

    let canonical = effective
        .iter()
        .map(|(flag, enabled)| format!("{flag}={}", if *enabled { '1' } else { '0' }))
        .collect::<Vec<_>>()
        .join(";");
    let digest = Sha256::digest(canonical.as_bytes());
    format!("v1-{:x}", digest)[..19].to_owned()
}
