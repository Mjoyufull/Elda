use std::collections::BTreeMap;

use crate::error::RecipeError;
use crate::model::{
    BuildDefinition, GitHubReleaseAssetDefinition, LuaValue, PackageDefinition, ProfilePolicy,
    ScalarValue, SourceDefinition, SourceLaneDefinition,
};

use super::fields::{
    get_dependency_entries, get_optional_boolean, get_optional_integer, get_optional_string,
    get_optional_string_array, get_optional_table, get_optional_value, get_required_integer,
    get_required_string, get_required_string_array, get_required_table, integer_to_u64,
    scalar_from_lua,
};

pub(super) fn map_package_definition(
    root: BTreeMap<String, LuaValue>,
) -> Result<PackageDefinition, RecipeError> {
    let name = get_required_string(&root, "name")?;
    let epoch = get_optional_integer(&root, "epoch")?.unwrap_or(0);
    let version = get_required_string(&root, "version")?;
    let rel = get_required_integer(&root, "rel")?;
    let arch = get_required_string_array(&root, "arch")?;
    let kind = get_required_string(&root, "kind")?;
    let source = map_source_definition(get_required_table(&root, "source")?)?;
    let build = get_optional_table(&root, "build")?
        .map(map_build_definition)
        .transpose()?;

    Ok(PackageDefinition {
        name,
        description: get_optional_string(&root, "description")?,
        licenses: get_optional_string_array(&root, "licenses")?.unwrap_or_default(),
        upstream: get_optional_string(&root, "upstream")?,
        epoch: integer_to_u64(epoch, "epoch")?,
        version,
        rel: integer_to_u64(rel, "rel")?,
        arch,
        kind,
        source,
        depends: get_dependency_entries(&root, "depends")?,
        makedepends: get_dependency_entries(&root, "makedepends")?,
        checkdepends: get_dependency_entries(&root, "checkdepends")?,
        recommends: get_dependency_entries(&root, "recommends")?,
        suggests: get_dependency_entries(&root, "suggests")?,
        supplements: get_dependency_entries(&root, "supplements")?,
        enhances: get_dependency_entries(&root, "enhances")?,
        provides: get_optional_string_array(&root, "provides")?.unwrap_or_default(),
        conflicts: get_optional_string_array(&root, "conflicts")?.unwrap_or_default(),
        replaces: get_optional_string_array(&root, "replaces")?.unwrap_or_default(),
        conffiles: get_optional_string_array(&root, "conffiles")?.unwrap_or_default(),
        sysusers: get_optional_value(&root, "sysusers"),
        tmpfiles: get_optional_value(&root, "tmpfiles"),
        alternatives: get_optional_value(&root, "alternatives"),
        hooks: get_optional_value(&root, "hooks"),
        provider_assets: get_optional_value(&root, "provider_assets"),
        flags_default: get_optional_value(&root, "flags_default"),
        flags_allowed: get_optional_value(&root, "flags_allowed"),
        flags_implies: get_optional_value(&root, "flags_implies"),
        flags_conflicts: get_optional_value(&root, "flags_conflicts"),
        subpackages: get_optional_value(&root, "subpackages"),
        profile: get_optional_table(&root, "profile")?
            .map(map_profile_policy)
            .transpose()?,
        build,
        has_build_table: root.contains_key("build"),
    })
}

fn map_source_definition(
    source: BTreeMap<String, LuaValue>,
) -> Result<SourceDefinition, RecipeError> {
    if source.contains_key("lanes") {
        return map_multi_lane_source_definition(source);
    }

    let kind = get_required_string(&source, "kind")?;
    let github_release_assets = source_github_release_assets(&source)?;
    let fields = source_fields(source)?;

    Ok(SourceDefinition::single_lane_with_assets(
        kind,
        fields,
        github_release_assets,
    ))
}

fn map_multi_lane_source_definition(
    source: BTreeMap<String, LuaValue>,
) -> Result<SourceDefinition, RecipeError> {
    if source.contains_key("kind") {
        return Err(RecipeError::Parse(
            "source cannot define both `kind` and `lanes`".to_owned(),
        ));
    }

    let default_lane = get_optional_string(&source, "default_lane")?;
    let lanes_table = get_required_table(&source, "lanes")?;
    validate_multi_lane_source_shape(&source)?;

    let mut lanes = BTreeMap::new();
    for (lane_name, lane_value) in lanes_table {
        let LuaValue::Table(table) = lane_value else {
            return Err(RecipeError::Parse(format!(
                "source.lanes.{lane_name} must be a table"
            )));
        };
        lanes.insert(lane_name, map_source_lane_definition(table)?);
    }

    Ok(SourceDefinition {
        kind: String::new(),
        fields: BTreeMap::new(),
        github_release_assets: BTreeMap::new(),
        default_lane,
        lanes,
    })
}

fn map_source_lane_definition(
    source: BTreeMap<String, LuaValue>,
) -> Result<SourceLaneDefinition, RecipeError> {
    let kind = get_required_string(&source, "kind")?;
    let github_release_assets = source_github_release_assets(&source)?;
    let fields = source_fields(source)?;

    Ok(SourceLaneDefinition {
        kind,
        fields,
        github_release_assets,
    })
}

fn map_build_definition(build: BTreeMap<String, LuaValue>) -> Result<BuildDefinition, RecipeError> {
    Ok(BuildDefinition {
        system: get_required_string(&build, "system")?,
        bins: get_optional_string_array(&build, "bins")?.unwrap_or_default(),
        features: get_optional_string_array(&build, "features")?.unwrap_or_default(),
        tests: get_optional_boolean(&build, "tests")?.unwrap_or(false),
    })
}

fn map_profile_policy(profile: BTreeMap<String, LuaValue>) -> Result<ProfilePolicy, RecipeError> {
    validate_profile_policy_keys(&profile)?;

    Ok(ProfilePolicy {
        native_arch: get_optional_string(&profile, "native_arch")?,
        foreign_arches: get_optional_string_array(&profile, "foreign_arches")?.unwrap_or_default(),
        init: get_optional_string(&profile, "init")?,
    })
}

fn source_fields(
    source: BTreeMap<String, LuaValue>,
) -> Result<BTreeMap<String, ScalarValue>, RecipeError> {
    let mut fields = BTreeMap::new();
    for (key, value) in source {
        if key == "kind" || key == "assets" {
            continue;
        }
        fields.insert(key.clone(), scalar_from_lua(&key, value)?);
    }
    Ok(fields)
}

fn source_github_release_assets(
    source: &BTreeMap<String, LuaValue>,
) -> Result<BTreeMap<String, GitHubReleaseAssetDefinition>, RecipeError> {
    let Some(assets) = get_optional_table(source, "assets")? else {
        return Ok(BTreeMap::new());
    };

    let mut parsed = BTreeMap::new();
    for (arch, value) in assets {
        let LuaValue::Table(fields) = value else {
            return Err(RecipeError::Parse(format!(
                "source.assets.{arch} must be a table"
            )));
        };
        parsed.insert(arch, map_github_release_asset_definition(fields)?);
    }

    Ok(parsed)
}

fn map_github_release_asset_definition(
    asset: BTreeMap<String, LuaValue>,
) -> Result<GitHubReleaseAssetDefinition, RecipeError> {
    Ok(GitHubReleaseAssetDefinition {
        asset: get_required_string(&asset, "asset")?,
        sha256: get_required_string(&asset, "sha256")?,
        binary: get_optional_string(&asset, "binary")?,
        strip_components: get_optional_integer(&asset, "strip_components")?,
        subdir: get_optional_string(&asset, "subdir")?,
        rename: get_optional_string(&asset, "rename")?,
    })
}

fn validate_multi_lane_source_shape(
    source: &BTreeMap<String, LuaValue>,
) -> Result<(), RecipeError> {
    for key in source.keys() {
        if key == "default_lane" || key == "lanes" {
            continue;
        }
        return Err(RecipeError::Parse(format!(
            "source field `{key}` is not allowed alongside `source.lanes`"
        )));
    }

    Ok(())
}

fn validate_profile_policy_keys(profile: &BTreeMap<String, LuaValue>) -> Result<(), RecipeError> {
    for key in profile.keys() {
        if key == "native_arch" || key == "foreign_arches" || key == "init" {
            continue;
        }
        return Err(RecipeError::Parse(format!(
            "profile field `{key}` is not supported in the current declarative slice"
        )));
    }

    Ok(())
}
