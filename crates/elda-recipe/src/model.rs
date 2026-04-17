use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;

pub const SOURCE_LANE_SOURCE: &str = "source";
pub const SOURCE_LANE_BINARY: &str = "binary";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecipeDocument {
    pub path: PathBuf,
    pub package: PackageDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageDefinition {
    pub name: String,
    pub epoch: u64,
    pub version: String,
    pub rel: u64,
    pub arch: Vec<String>,
    pub kind: String,
    pub source: SourceDefinition,
    pub depends: Vec<DependencyEntry>,
    pub makedepends: Vec<DependencyEntry>,
    pub checkdepends: Vec<DependencyEntry>,
    pub recommends: Vec<DependencyEntry>,
    pub suggests: Vec<DependencyEntry>,
    pub supplements: Vec<DependencyEntry>,
    pub enhances: Vec<DependencyEntry>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub conffiles: Vec<String>,
    pub sysusers: Option<LuaValue>,
    pub tmpfiles: Option<LuaValue>,
    pub alternatives: Option<LuaValue>,
    pub hooks: Option<LuaValue>,
    pub flags_default: Option<LuaValue>,
    pub flags_allowed: Option<LuaValue>,
    pub flags_implies: Option<LuaValue>,
    pub flags_conflicts: Option<LuaValue>,
    pub subpackages: Option<LuaValue>,
    pub build: Option<BuildDefinition>,
    pub has_build_table: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceDefinition {
    pub kind: String,
    pub fields: BTreeMap<String, ScalarValue>,
    pub github_release_assets: BTreeMap<String, GitHubReleaseAssetDefinition>,
    pub default_lane: Option<String>,
    pub lanes: BTreeMap<String, SourceLaneDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceLaneDefinition {
    pub kind: String,
    pub fields: BTreeMap<String, ScalarValue>,
    pub github_release_assets: BTreeMap<String, GitHubReleaseAssetDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitHubReleaseAssetDefinition {
    pub asset: String,
    pub sha256: String,
    pub binary: Option<String>,
    pub strip_components: Option<i64>,
    pub subdir: Option<String>,
    pub rename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildDefinition {
    pub system: String,
    pub bins: Vec<String>,
    pub features: Vec<String>,
    pub tests: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyEntry {
    Constraint(String),
    AnyOf(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScalarValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum LuaValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<LuaValue>),
    Table(BTreeMap<String, LuaValue>),
}

impl SourceDefinition {
    #[must_use]
    pub fn single_lane(kind: String, fields: BTreeMap<String, ScalarValue>) -> Self {
        Self::single_lane_with_assets(kind, fields, BTreeMap::new())
    }

    #[must_use]
    pub fn single_lane_with_assets(
        kind: String,
        fields: BTreeMap<String, ScalarValue>,
        github_release_assets: BTreeMap<String, GitHubReleaseAssetDefinition>,
    ) -> Self {
        Self {
            kind,
            fields,
            github_release_assets,
            default_lane: None,
            lanes: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn is_multi_lane(&self) -> bool {
        !self.lanes.is_empty()
    }

    #[must_use]
    pub fn available_lanes(&self) -> Vec<String> {
        if self.is_multi_lane() {
            return self.lanes.keys().cloned().collect();
        }

        infer_lane_name(&self.kind)
            .map(|lane| vec![lane.to_owned()])
            .unwrap_or_default()
    }

    #[must_use]
    pub fn lane_definition(&self, lane: &str) -> Option<SourceLaneDefinition> {
        if self.is_multi_lane() {
            return self.lanes.get(lane).cloned();
        }

        infer_lane_name(&self.kind)
            .filter(|inferred| *inferred == lane)
            .map(|_| SourceLaneDefinition {
                kind: self.kind.clone(),
                fields: self.fields.clone(),
                github_release_assets: self.github_release_assets.clone(),
            })
    }
}

#[must_use]
pub fn infer_lane_name(kind: &str) -> Option<&'static str> {
    match kind {
        "git" | "nix_flake" | "gentoo_overlay" => Some(SOURCE_LANE_SOURCE),
        "url_archive" | "github_release" => Some(SOURCE_LANE_BINARY),
        _ => None,
    }
}
