use std::path::{Path, PathBuf};

use clap::Args;

#[derive(Debug, Args)]
#[command(group(
    clap::ArgGroup::new("install-preference")
        .args(["prefer_source", "prefer_binary"])
        .multiple(false)
))]
pub(super) struct InstallArgs {
    #[arg(
        value_name = "TARGET",
        help = "Package names, local recipes, or git targets"
    )]
    pub(super) targets: Vec<String>,
    #[arg(
        long = "use",
        value_name = "+FLAG,-FLAG",
        help = "Apply one-shot package flag overrides"
    )]
    pub(super) use_flags: Vec<String>,
    #[arg(
        long = "source-option",
        value_name = "N",
        help = "Select one ranked source option from list-options output"
    )]
    pub(super) source_option: Option<usize>,
    #[arg(
        long = "strategy",
        value_name = "NAME",
        help = "Prefer one metadata strategy for link detection"
    )]
    pub(super) strategy: Option<String>,
    #[arg(
        long = "to-branch",
        value_name = "BRANCH",
        help = "Use one git branch for ad hoc git metadata"
    )]
    pub(super) to_branch: Option<String>,
    #[arg(
        long = "to-tag",
        value_name = "TAG",
        help = "Use one git tag for ad hoc git metadata"
    )]
    pub(super) to_tag: Option<String>,
    #[arg(
        long = "to-rev",
        value_name = "REV",
        help = "Use one exact git revision for ad hoc git metadata"
    )]
    pub(super) to_rev: Option<String>,
    #[arg(
        long = "pick-tag",
        help = "Interactively choose a git tag before planning the install"
    )]
    pub(super) pick_tag: bool,
    #[arg(
        long = "provider",
        value_name = "VIRTUAL=PACKAGE",
        help = "Resolve one ambiguous virtual provider explicitly"
    )]
    pub(super) provider: Vec<String>,
    #[arg(
        long = "prefer-source",
        help = "Prefer the source lane when normal selection applies"
    )]
    pub(super) prefer_source: bool,
    #[arg(
        long = "prefer-binary",
        help = "Prefer the binary lane when normal selection applies"
    )]
    pub(super) prefer_binary: bool,
    #[arg(long, help = "Replace existing local recipes during bulk import")]
    pub(super) replace: bool,
    #[arg(
        long,
        num_args = 1..,
        value_name = "PKG",
        help = "Exclude packages from bulk import/replace; must appear after other flags. Each value may be comma-separated; additional tokens name more packages"
    )]
    pub(super) exclude: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct TargetsArgs {
    #[arg(value_name = "TARGET", help = "Target package names or git inputs")]
    pub(super) targets: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct InstallTargetsArgs {
    #[arg(value_name = "TARGET", help = "Target package names or git inputs")]
    pub(super) targets: Vec<String>,
    #[arg(
        long = "use",
        value_name = "+FLAG,-FLAG",
        help = "Apply one-shot package flag overrides"
    )]
    pub(super) use_flags: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct ProfileApplyArgs {
    #[arg(value_name = "PROFILE", help = "Profile anchor package names")]
    pub(super) targets: Vec<String>,
    #[arg(long, help = "Set the active init-provider family")]
    pub(super) init: Option<String>,
    #[arg(
        long = "native-arch",
        value_name = "ARCH",
        help = "Set the active native architecture"
    )]
    pub(super) native_arch: Option<String>,
    #[arg(
        long = "foreign-arch",
        value_name = "ARCH",
        help = "Enable one foreign architecture or multilib architecture"
    )]
    pub(super) foreign_arches: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct OptionalTargetsArgs {
    #[arg(value_name = "PKG", help = "Optional installed package names")]
    pub(super) targets: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct ArchListArgs {
    #[arg(value_name = "ARCH", help = "Canonical Elda architecture labels")]
    pub(super) arches: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct UpgradeArgs {
    #[arg(value_name = "PKG", help = "Optional installed package names")]
    pub(super) targets: Vec<String>,
    #[arg(
        long = "refresh-weak-deps",
        help = "Allow newly introduced weak dependencies during upgrade"
    )]
    pub(super) refresh_weak_deps: bool,
    #[arg(
        long = "rebuild-variant-drift",
        help = "Rebuild installed packages whose effective flag variant differs from the resolved one"
    )]
    pub(super) rebuild_variant_drift: bool,
    #[arg(
        long = "to-branch",
        value_name = "BRANCH",
        help = "Move selected ad hoc git package(s) to one branch"
    )]
    pub(super) to_branch: Option<String>,
    #[arg(
        long = "to-tag",
        value_name = "TAG",
        help = "Move selected ad hoc git package(s) to one tag"
    )]
    pub(super) to_tag: Option<String>,
    #[arg(
        long = "to-rev",
        value_name = "REV",
        help = "Move selected ad hoc git package(s) to one exact revision"
    )]
    pub(super) to_rev: Option<String>,
    #[arg(
        long = "pick-tag",
        help = "Interactively choose a git tag before planning the upgrade"
    )]
    pub(super) pick_tag: bool,
}

#[derive(Debug, Args)]
pub(super) struct RemoveArgs {
    #[arg(value_name = "PKG", help = "Installed package names to remove")]
    pub(super) packages: Vec<String>,
    #[arg(long, help = "Also remove reverse dependencies that become invalid")]
    pub(super) cascade: bool,
    #[arg(
        long = "purge-conffiles",
        help = "Drop preserved *.eldasave configuration state"
    )]
    pub(super) purge_conffiles: bool,
}

#[derive(Debug, Args)]
pub(super) struct SearchArgs {
    #[arg(help = "Substring query or regex pattern")]
    pub(super) query: String,
    #[arg(long, help = "Treat the query as a regular expression")]
    pub(super) regex: bool,
    #[arg(
        long,
        help = "Interactive selection prompt; can trigger install from matches in human mode"
    )]
    pub(super) interactive: bool,
}

#[derive(Debug, Args)]
pub(super) struct PackageArg {
    #[arg(help = "Installed or synced package name")]
    pub(super) package: String,
}

#[derive(Debug, Args, Default)]
pub(super) struct ListArgs {
    #[arg(long, help = "Show only explicitly requested packages")]
    pub(super) explicit: bool,
    #[arg(long, help = "Show only dependency packages")]
    pub(super) deps: bool,
    #[arg(long, help = "Show only held packages")]
    pub(super) held: bool,
    #[arg(long, help = "Show only pinned packages")]
    pub(super) pinned: bool,
    #[arg(
        long = "source-kind",
        help = "Show only packages with this source kind"
    )]
    pub(super) source_kind: Option<String>,
}

#[derive(Debug, Args, Default)]
pub(super) struct ListDetailArgs {
    #[arg(help = "Optional installed package names to restrict the listing")]
    pub(super) packages: Vec<String>,
    #[command(flatten)]
    pub(super) filters: ListArgs,
}

#[derive(Debug, Args)]
pub(super) struct RdepsArgs {
    #[arg(help = "Installed package name")]
    pub(super) package: String,
    #[arg(long, help = "Walk reverse dependencies transitively")]
    pub(super) all: bool,
    #[arg(long, help = "Include weak dependency edges")]
    pub(super) weak: bool,
}

#[derive(Debug, Args)]
pub(super) struct HoldArgs {
    #[arg(help = "Installed package name")]
    pub(super) package: String,
    #[arg(long, help = "Optional source selector recorded with the hold")]
    pub(super) source: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct AdoptArgs {
    #[arg(long, help = "Source package manager adapter name")]
    pub(super) from: String,
    #[arg(help = "Package name to adopt")]
    pub(super) package: String,
}

#[derive(Debug, Args)]
pub(super) struct DowngradeArgs {
    #[arg(help = "Installed package name")]
    pub(super) package: String,
    #[arg(help = "Optional older archived version to request explicitly")]
    pub(super) version: Option<String>,
    #[arg(
        long = "to-tag",
        value_name = "TAG",
        help = "Rebuild an ad hoc git package from one older tag"
    )]
    pub(super) to_tag: Option<String>,
    #[arg(
        long = "to-rev",
        value_name = "REV",
        help = "Rebuild an ad hoc git package from one exact revision"
    )]
    pub(super) to_rev: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct DiffArgs {
    #[arg(help = "Installed package name")]
    pub(super) package: String,
    #[arg(long, help = "Compare against the next candidate package manifest")]
    pub(super) candidate: bool,
}

#[derive(Debug, Args)]
pub(super) struct RollbackArgs {
    #[arg(help = "Archived state id; defaults to the previous archived state")]
    pub(super) state_id: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct VendorAddArgs {
    #[arg(help = "Local package name to create or update")]
    pub(super) package: String,
    #[arg(help = "GitHub release spec, URL, or local file path")]
    pub(super) source: String,
    #[arg(long, help = "Executable name inside an archive payload")]
    pub(super) binary: Option<String>,
    #[arg(long, help = "Explicit release asset name when detection is ambiguous")]
    pub(super) asset: Option<String>,
    #[arg(long, help = "Replace existing vendor recipe metadata")]
    pub(super) replace: bool,
}

#[derive(Debug, Args)]
pub(super) struct VendorImportArgs {
    #[arg(help = "Manifest or lock path")]
    pub(super) path: PathBuf,
    #[arg(long, help = "Replace existing vendor recipe metadata")]
    pub(super) replace: bool,
}

#[derive(Debug, Args)]
pub(super) struct VendorExportArgs {
    #[arg(help = "Output manifest or lock path")]
    pub(super) path: PathBuf,
    #[arg(value_name = "PKG", help = "Vendor package names to export")]
    pub(super) packages: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct ValueArg {
    pub(super) value: String,
}

#[derive(Debug, Args)]
pub(super) struct OptionalValueArg {
    pub(super) value: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct PathArg {
    pub(super) value: String,
}

#[derive(Debug, Args)]
pub(super) struct OptionalPackageArg {
    pub(super) package: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct OptionalPathArg {
    pub(super) path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(super) struct PathBufArg {
    pub(super) path: PathBuf,
}

pub(super) fn push_flag(operands: &mut Vec<String>, flag: &str, enabled: bool) {
    if enabled {
        operands.push(flag.to_owned());
    }
}

pub(super) fn push_optional(operands: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        operands.push(flag.to_owned());
        operands.push(value.to_owned());
    }
}

pub(super) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
