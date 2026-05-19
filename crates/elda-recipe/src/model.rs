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
    pub description: Option<String>,
    pub licenses: Vec<String>,
    pub upstream: Option<String>,
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
    pub provider_assets: Option<LuaValue>,
    pub flags_default: Option<LuaValue>,
    pub flags_allowed: Option<LuaValue>,
    pub flags_implies: Option<LuaValue>,
    pub flags_conflicts: Option<LuaValue>,
    pub flags_descriptions: Option<LuaValue>,
    pub flags_required_one_of: Option<LuaValue>,
    pub flags_required_at_most_one: Option<LuaValue>,
    pub flags_required_any_of: Option<LuaValue>,
    pub subpackages: Option<LuaValue>,
    pub profile: Option<ProfilePolicy>,
    pub build: Option<BuildDefinition>,
    pub has_build_table: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfilePolicy {
    pub native_arch: Option<String>,
    pub foreign_arches: Vec<String>,
    pub init: Option<String>,
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
    pub signature: Option<String>,
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
pub struct DependencyEntry {
    pub body: DependencyBody,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<FlagPredicate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyBody {
    Constraint(String),
    AnyOf(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlagPredicate {
    pub raw: String,
    pub atoms: Vec<FlagPredicateAtom>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlagPredicateAtom {
    pub flag: String,
    pub expected: bool,
}

impl DependencyEntry {
    #[must_use]
    pub fn constraint(value: impl Into<String>) -> Self {
        Self {
            body: DependencyBody::Constraint(value.into()),
            when: None,
        }
    }

    #[must_use]
    pub fn any_of<I: IntoIterator<Item = String>>(values: I) -> Self {
        Self {
            body: DependencyBody::AnyOf(values.into_iter().collect()),
            when: None,
        }
    }

    #[must_use]
    pub fn with_when(mut self, when: Option<FlagPredicate>) -> Self {
        self.when = when;
        self
    }

    #[must_use]
    pub fn referenced_flags(&self) -> Vec<&str> {
        self.when
            .as_ref()
            .map(|predicate| {
                predicate
                    .atoms
                    .iter()
                    .map(|atom| atom.flag.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl FlagPredicate {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let mut atoms = Vec::new();
        for token in raw.split(',') {
            let token = token.trim();
            if token.is_empty() {
                continue;
            }
            let (expected, name) = if let Some(name) = token.strip_prefix('+') {
                (true, name)
            } else if let Some(name) = token.strip_prefix('-') {
                (false, name)
            } else {
                return Err(format!(
                    "flag predicate token `{token}` must start with `+` or `-`"
                ));
            };
            let name = name.trim();
            if name.is_empty() {
                return Err(format!(
                    "flag predicate `{raw}` contains an empty flag name"
                ));
            }
            atoms.push(FlagPredicateAtom {
                flag: name.to_owned(),
                expected,
            });
        }
        if atoms.is_empty() {
            return Err(format!(
                "flag predicate `{raw}` must contain at least one `+flag` or `-flag` atom"
            ));
        }
        Ok(Self {
            raw: raw.to_owned(),
            atoms,
        })
    }

    #[must_use]
    pub fn evaluate(&self, effective: &BTreeMap<String, bool>) -> bool {
        self.atoms.iter().all(|atom| {
            let actual = effective.get(&atom.flag).copied().unwrap_or(false);
            actual == atom.expected
        })
    }
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
        "git" | "nix_flake" | "gentoo_overlay" | "aur_pkgbuild" | "xbps_template" => {
            Some(SOURCE_LANE_SOURCE)
        }
        "url_archive" | "github_release" | "release_asset" | "appimage" => Some(SOURCE_LANE_BINARY),
        _ => None,
    }
}
