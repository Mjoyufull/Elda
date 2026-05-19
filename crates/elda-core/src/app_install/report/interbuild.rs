use serde_json::json;

use crate::app::ResolvedInstallTarget;

pub(crate) fn interbuild_details(resolved: &ResolvedInstallTarget) -> Option<serde_json::Value> {
    match resolved.selected_source_kind.as_str() {
        "nix_flake" => Some(nix_flake_details(resolved)),
        "gentoo_overlay" => Some(gentoo_overlay_details(resolved)),
        "aur_pkgbuild" => Some(aur_pkgbuild_details(resolved)),
        "xbps_template" => Some(xbps_template_details(resolved)),
        _ => None,
    }
}

fn nix_flake_details(resolved: &ResolvedInstallTarget) -> serde_json::Value {
    let system = resolved
        .recipe
        .package
        .arch
        .iter()
        .filter_map(|arch| nix_system_for_arch(arch))
        .collect::<Vec<_>>();
    json!({
        "parser": "nix_flake",
        "engine": "static-flake-output-parser",
        "confidence": "bounded-static-output",
        "external_cli_required": false,
        "target": string_field(resolved, "installable").unwrap_or("default"),
        "candidate_systems": system,
        "lockfile": {
            "present": null,
            "locked_inputs": null,
            "allowed_input_kinds": ["git", "github", "tarball"]
        },
        "gentoo": null,
        "aur": null,
        "xbps": null,
    })
}

fn gentoo_overlay_details(resolved: &ResolvedInstallTarget) -> serde_json::Value {
    json!({
        "parser": "gentoo_overlay",
        "engine": "bounded-ebuild-metadata-parser",
        "confidence": "curated-eclass-and-simple-phase-subset",
        "external_cli_required": false,
        "target": string_field(resolved, "package").unwrap_or(&resolved.recipe.package.name),
        "candidate_systems": [],
        "lockfile": null,
        "gentoo": {
            "package": string_field(resolved, "package").unwrap_or(&resolved.recipe.package.name),
            "ebuild": null,
            "eapi": "8",
            "inherited_eclasses": [],
            "description": "",
            "homepage": "",
            "license": [],
            "src_uri": [],
            "slot": null,
            "depend": [],
            "rdepend": [],
            "bdepend": [],
            "iuse": [],
            "keywords": [],
            "phases": [],
            "phase_commands": []
        },
        "aur": null,
        "xbps": null,
    })
}

fn aur_pkgbuild_details(resolved: &ResolvedInstallTarget) -> serde_json::Value {
    json!({
        "parser": "aur_pkgbuild",
        "engine": "bounded-pkgbuild-metadata-parser",
        "confidence": "bounded-array-and-function-subset",
        "external_cli_required": false,
        "target": string_field(resolved, "pkgname").unwrap_or(&resolved.recipe.package.name),
        "candidate_systems": [],
        "lockfile": null,
        "gentoo": null,
        "aur": {
            "pkgname": string_field(resolved, "pkgname").unwrap_or(&resolved.recipe.package.name),
            "pkgver": string_field(resolved, "pkgver").unwrap_or(&resolved.recipe.package.version.to_string()),
            "pkgrel": "1",
            "epoch": null,
            "pkgdesc": "",
            "url": "",
            "license": [],
            "source": [],
            "arch_sources": [],
            "vcs_sources": [],
            "pkgver_function": false,
            "depends": [],
            "makedepends": [],
            "checkdepends": [],
            "optdepends": [],
            "provides": [],
            "conflicts": [],
            "replaces": [],
            "functions": [],
            "phase_commands": []
        },
        "xbps": null,
    })
}

fn xbps_template_details(resolved: &ResolvedInstallTarget) -> serde_json::Value {
    json!({
        "parser": "xbps_template",
        "engine": "bounded-xbps-template-parser",
        "confidence": "bounded-variable-and-function-subset",
        "external_cli_required": false,
        "target": string_field(resolved, "pkgname").unwrap_or(&resolved.recipe.package.name),
        "candidate_systems": [],
        "lockfile": null,
        "gentoo": null,
        "aur": null,
        "xbps": {
            "pkgname": string_field(resolved, "pkgname").unwrap_or(&resolved.recipe.package.name),
            "version": string_field(resolved, "version").unwrap_or(&resolved.recipe.package.version.to_string()),
            "revision": string_field(resolved, "revision").unwrap_or("1"),
            "short_desc": "",
            "homepage": "",
            "license": [],
            "distfiles": [],
            "checksum": [],
            "depends": [],
            "makedepends": [],
            "hostmakedepends": [],
            "checkdepends": [],
            "provides": [],
            "conflicts": [],
            "archs": [],
            "functions": [],
            "phase_commands": []
        }
    })
}

fn string_field<'a>(resolved: &'a ResolvedInstallTarget, key: &str) -> Option<&'a str> {
    match resolved.recipe.package.source.fields.get(key) {
        Some(elda_recipe::ScalarValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn nix_system_for_arch(arch: &str) -> Option<&'static str> {
    match arch {
        "amd64" => Some("x86_64-linux"),
        "i386" => Some("i686-linux"),
        "arm64" => Some("aarch64-linux"),
        "armhf" => Some("armv7l-linux"),
        "riscv64" => Some("riscv64-linux"),
        "ppc64le" => Some("powerpc64le-linux"),
        _ => None,
    }
}
