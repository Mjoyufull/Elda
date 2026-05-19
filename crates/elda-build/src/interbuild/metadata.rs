use serde::Serialize;

use super::nix::NixMetaReport;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterbuildReport {
    pub parser: &'static str,
    pub engine: &'static str,
    pub confidence: &'static str,
    pub external_cli_required: bool,
    pub target: Option<String>,
    pub candidate_systems: Vec<String>,
    pub lockfile: Option<LockfileReport>,
    pub nix_meta: Option<NixMetaReport>,
    pub gentoo: Option<GentooReport>,
    pub aur: Option<AurReport>,
    pub xbps: Option<XbpsReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LockfileReport {
    pub present: bool,
    pub locked_inputs: usize,
    pub allowed_input_kinds: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PhaseCommandReport {
    pub phase: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArchSourceReport {
    pub arch: String,
    pub source: Vec<String>,
    pub checksum: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AurReport {
    pub pkgname: String,
    pub pkgver: String,
    pub pkgrel: String,
    pub epoch: Option<String>,
    pub pkgdesc: String,
    pub url: String,
    pub license: Vec<String>,
    pub source: Vec<String>,
    pub arch_sources: Vec<ArchSourceReport>,
    pub vcs_sources: Vec<String>,
    pub pkgver_function: bool,
    pub depends: Vec<String>,
    pub makedepends: Vec<String>,
    pub checkdepends: Vec<String>,
    pub optdepends: Vec<String>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub functions: Vec<String>,
    pub phase_commands: Vec<PhaseCommandReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct XbpsReport {
    pub pkgname: String,
    pub version: String,
    pub revision: String,
    pub short_desc: String,
    pub homepage: String,
    pub license: Vec<String>,
    pub distfiles: Vec<String>,
    pub checksum: Vec<String>,
    pub depends: Vec<String>,
    pub makedepends: Vec<String>,
    pub hostmakedepends: Vec<String>,
    pub checkdepends: Vec<String>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub archs: Vec<String>,
    pub build_style: Option<String>,
    pub configure_args: Option<String>,
    pub functions: Vec<String>,
    pub phase_commands: Vec<PhaseCommandReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GentooReport {
    pub package: String,
    pub ebuild: String,
    pub eapi: String,
    pub inherited_eclasses: Vec<String>,
    pub description: String,
    pub homepage: String,
    pub license: Vec<String>,
    pub src_uri: Vec<String>,
    pub slot: Option<String>,
    pub depend: Vec<String>,
    pub rdepend: Vec<String>,
    pub bdepend: Vec<String>,
    pub iuse: Vec<String>,
    pub keywords: Vec<String>,
    pub phases: Vec<String>,
    pub phase_commands: Vec<PhaseCommandReport>,
    pub gpkg_used: bool,
    pub gpkg_use: Vec<String>,
}

impl InterbuildReport {
    pub(crate) fn nix_flake(
        target: String,
        candidate_systems: Vec<String>,
        lockfile: LockfileReport,
        nix_meta: NixMetaReport,
    ) -> Self {
        Self {
            parser: "nix_flake",
            engine: "static-flake-output-parser",
            confidence: "bounded-static-output",
            external_cli_required: false,
            target: Some(target),
            candidate_systems,
            lockfile: Some(lockfile),
            nix_meta: Some(nix_meta),
            gentoo: None,
            aur: None,
            xbps: None,
        }
    }

    pub(crate) fn gentoo_overlay(report: GentooReport) -> Self {
        Self {
            parser: "gentoo_overlay",
            engine: "bounded-ebuild-metadata-parser",
            confidence: "curated-eclass-and-simple-phase-subset",
            external_cli_required: false,
            target: Some(report.package.clone()),
            candidate_systems: Vec::new(),
            lockfile: None,
            nix_meta: None,
            gentoo: Some(report),
            aur: None,
            xbps: None,
        }
    }

    pub(crate) fn aur_pkgbuild(report: AurReport) -> Self {
        Self {
            parser: "aur_pkgbuild",
            engine: "bounded-pkgbuild-metadata-parser",
            confidence: "bounded-array-and-function-subset",
            external_cli_required: false,
            target: Some(report.pkgname.clone()),
            candidate_systems: Vec::new(),
            lockfile: None,
            nix_meta: None,
            gentoo: None,
            aur: Some(report),
            xbps: None,
        }
    }

    pub(crate) fn xbps_template(report: XbpsReport) -> Self {
        Self {
            parser: "xbps_template",
            engine: "bounded-xbps-template-parser",
            confidence: "bounded-variable-and-function-subset",
            external_cli_required: false,
            target: Some(report.pkgname.clone()),
            candidate_systems: Vec::new(),
            lockfile: None,
            nix_meta: None,
            gentoo: None,
            aur: None,
            xbps: Some(report),
        }
    }
}

impl LockfileReport {
    pub(crate) fn absent() -> Self {
        Self {
            present: false,
            locked_inputs: 0,
            allowed_input_kinds: vec!["git", "github", "tarball"],
        }
    }

    pub(crate) fn present(locked_inputs: usize) -> Self {
        Self {
            present: true,
            locked_inputs,
            allowed_input_kinds: vec!["git", "github", "tarball"],
        }
    }
}
