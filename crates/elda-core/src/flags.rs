use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::app::{AppContext, ParsedInstallRequest};
use crate::config::FlagsConfig;
use crate::error::CoreError;
use elda_recipe::{LuaValue, PackageDefinition};
use elda_types::NamedConstraint;

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
    pub(crate) descriptions: BTreeMap<String, String>,
    pub(crate) cardinality_groups: Vec<CardinalityGroup>,
    pub(crate) package_flag_layers: Vec<PackageFlagLayer>,
    pub(crate) variant_id: String,
    pub(crate) customized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardinalityGroup {
    pub(crate) kind: CardinalityKind,
    pub(crate) name: String,
    pub(crate) members: Vec<String>,
    pub(crate) selected: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardinalityKind {
    OneOf,
    AtMostOne,
    AnyOf,
}

impl CardinalityKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            CardinalityKind::OneOf => "one-of",
            CardinalityKind::AtMostOne => "at-most-one",
            CardinalityKind::AnyOf => "any-of",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageFlagLayer {
    pub(crate) source: String,
    pub(crate) flags: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PackageFlagPolicy {
    allowed: BTreeSet<String>,
    defaults: BTreeMap<String, bool>,
    implies: BTreeMap<String, Vec<String>>,
    conflicts: BTreeMap<String, Vec<String>>,
    descriptions: BTreeMap<String, String>,
    one_of: BTreeMap<String, Vec<String>>,
    at_most_one: BTreeMap<String, Vec<String>>,
    any_of: BTreeMap<String, Vec<String>>,
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
        let package_flag_layers = collect_package_flag_layers(&self.config.flags, package);
        let package_flags = merge_layered_flags(&package_flag_layers);
        let cli_flags = request.cli_flag_overrides.clone();

        validate_flag_names(&policy.allowed, "global flag", &global_flags)?;
        validate_flag_names(&policy.allowed, "profile flag", &profile_flags)?;
        for layer in &package_flag_layers {
            validate_flag_names(
                &policy.allowed,
                &format!("package flag (`{}`)", layer.source),
                &layer.flags,
            )?;
        }
        validate_flag_names(&policy.allowed, "CLI flag", &cli_flags)?;

        let mut effective_flags = normalized_flag_map(&policy.allowed, &policy.defaults);
        apply_flag_overrides(&mut effective_flags, &global_flags);
        apply_flag_overrides(&mut effective_flags, &profile_flags);
        apply_flag_overrides(&mut effective_flags, &package_flags);
        apply_flag_overrides(&mut effective_flags, &cli_flags);
        apply_implied_flags(&policy, &mut effective_flags)?;
        validate_flag_conflicts(package, &policy, &effective_flags)?;
        let cardinality_groups = evaluate_flag_cardinality(package, &policy, &effective_flags)?;

        let default_flags = normalized_flag_map(&policy.allowed, &policy.defaults);
        let customized = effective_flags != default_flags;
        let variant_id = if customized {
            variant_id_for_flags(&effective_flags)
        } else {
            "default".to_owned()
        };

        Ok(ResolvedFlagState {
            active_profiles: profile.active_profiles,
            allowed_flags: policy.allowed.iter().cloned().collect(),
            default_flags: default_flags.clone(),
            global_flags,
            profile_flags,
            package_flags,
            cli_flags,
            effective_flags: effective_flags.clone(),
            descriptions: policy.descriptions,
            cardinality_groups,
            package_flag_layers,
            variant_id,
            customized,
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
        let descriptions =
            parse_string_table(package.flags_descriptions.as_ref(), "flags_descriptions")?;
        let one_of = parse_string_list_table(
            package.flags_required_one_of.as_ref(),
            "flags_required_one_of",
        )?;
        let at_most_one = parse_string_list_table(
            package.flags_required_at_most_one.as_ref(),
            "flags_required_at_most_one",
        )?;
        let any_of = parse_string_list_table(
            package.flags_required_any_of.as_ref(),
            "flags_required_any_of",
        )?;

        validate_referenced_flags(&allowed, "flags_implies", &implies)?;
        validate_referenced_flags(&allowed, "flags_conflicts", &conflicts)?;
        validate_cardinality_members(&allowed, "flags_required_one_of", &one_of)?;
        validate_cardinality_members(&allowed, "flags_required_at_most_one", &at_most_one)?;
        validate_cardinality_members(&allowed, "flags_required_any_of", &any_of)?;
        for flag in descriptions.keys() {
            if !allowed.contains(flag) {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!("flags_descriptions.{flag} references an undeclared flag"),
                )));
            }
        }

        Ok(Self {
            allowed,
            defaults,
            implies,
            conflicts,
            descriptions,
            one_of,
            at_most_one,
            any_of,
        })
    }
}

fn parse_string_table(
    value: Option<&LuaValue>,
    field: &str,
) -> Result<BTreeMap<String, String>, CoreError> {
    let Some(LuaValue::Table(table)) = value else {
        return Ok(BTreeMap::new());
    };

    let mut parsed = BTreeMap::new();
    for (name, value) in table {
        let LuaValue::String(text) = value else {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("{field}.{name} must be a string"),
            )));
        };
        parsed.insert(name.clone(), text.clone());
    }

    Ok(parsed)
}

fn validate_cardinality_members(
    allowed: &BTreeSet<String>,
    field: &str,
    table: &BTreeMap<String, Vec<String>>,
) -> Result<(), CoreError> {
    for (group, members) in table {
        if members.len() < 2 {
            return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                format!("{field}.{group} must list at least two flag names"),
            )));
        }
        for member in members {
            if !allowed.contains(member) {
                return Err(CoreError::Recipe(elda_recipe::RecipeError::InvalidInput(
                    format!("{field}.{group} references undeclared flag `{member}`"),
                )));
            }
        }
    }
    Ok(())
}

fn evaluate_flag_cardinality(
    package: &PackageDefinition,
    policy: &PackageFlagPolicy,
    effective: &BTreeMap<String, bool>,
) -> Result<Vec<CardinalityGroup>, CoreError> {
    let mut groups = Vec::new();

    for (name, members) in &policy.one_of {
        let selected = members
            .iter()
            .filter(|flag| effective.get(*flag).copied().unwrap_or(false))
            .cloned()
            .collect::<Vec<_>>();
        if selected.len() != 1 {
            return Err(CoreError::Operator(format!(
                "package `{}` flag group `{name}` requires exactly one of: {} (selected: {})",
                package.name,
                members.join(", "),
                fmt_selected(&selected),
            )));
        }
        groups.push(CardinalityGroup {
            kind: CardinalityKind::OneOf,
            name: name.clone(),
            members: members.clone(),
            selected,
        });
    }

    for (name, members) in &policy.at_most_one {
        let selected = members
            .iter()
            .filter(|flag| effective.get(*flag).copied().unwrap_or(false))
            .cloned()
            .collect::<Vec<_>>();
        if selected.len() > 1 {
            return Err(CoreError::Operator(format!(
                "package `{}` flag group `{name}` allows at most one of: {} (selected: {})",
                package.name,
                members.join(", "),
                fmt_selected(&selected),
            )));
        }
        groups.push(CardinalityGroup {
            kind: CardinalityKind::AtMostOne,
            name: name.clone(),
            members: members.clone(),
            selected,
        });
    }

    for (name, members) in &policy.any_of {
        let selected = members
            .iter()
            .filter(|flag| effective.get(*flag).copied().unwrap_or(false))
            .cloned()
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err(CoreError::Operator(format!(
                "package `{}` flag group `{name}` requires at least one of: {}",
                package.name,
                members.join(", "),
            )));
        }
        groups.push(CardinalityGroup {
            kind: CardinalityKind::AnyOf,
            name: name.clone(),
            members: members.clone(),
            selected,
        });
    }

    Ok(groups)
}

fn fmt_selected(selected: &[String]) -> String {
    if selected.is_empty() {
        "none".to_owned()
    } else {
        selected.join(", ")
    }
}

fn collect_package_flag_layers(
    config: &FlagsConfig,
    package: &PackageDefinition,
) -> Vec<PackageFlagLayer> {
    let mut layers = Vec::new();

    let mut keys: Vec<&String> = config.package.keys().collect();
    keys.sort();
    let mut atom_layers = Vec::new();
    for key in keys {
        let flags = config.package.get(key).cloned().unwrap_or_default();
        if flags.is_empty() {
            continue;
        }
        if key == &package.name {
            layers.push(PackageFlagLayer {
                source: package.name.clone(),
                flags,
            });
            continue;
        }
        let Ok(constraint) = NamedConstraint::parse_dependency(key) else {
            continue;
        };
        if constraint.name != package.name {
            continue;
        }
        let actual = elda_types::ConstraintVersion::from_parts(
            package.epoch,
            package.version.clone(),
            Some(package.rel),
        );
        if constraint.matches_version(&actual) {
            atom_layers.push(PackageFlagLayer {
                source: key.clone(),
                flags,
            });
        }
    }

    layers.extend(atom_layers);
    layers
}

fn merge_layered_flags(layers: &[PackageFlagLayer]) -> BTreeMap<String, bool> {
    let mut merged = BTreeMap::new();
    for layer in layers {
        for (flag, enabled) in &layer.flags {
            merged.insert(flag.clone(), *enabled);
        }
    }
    merged
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
