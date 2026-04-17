use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use elda_core::{PrivilegeProvider, PrivilegeRequest};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedProvider {
    pub(super) requested: PrivilegeProvider,
    pub(super) effective: PrivilegeProvider,
    pub(super) binary_name: &'static str,
    pub(super) binary_path: PathBuf,
}

const AUTO_PROVIDER_ORDER: &[(PrivilegeProvider, &str)] = &[
    (PrivilegeProvider::Doas, "doas"),
    (PrivilegeProvider::Sudo, "sudo"),
    (PrivilegeProvider::Run0, "run0"),
    (PrivilegeProvider::Su, "su"),
];

pub(super) fn resolve_provider(
    request: &PrivilegeRequest,
    search_path: Option<OsString>,
) -> anyhow::Result<ResolvedProvider> {
    let search_path = search_path.as_deref();

    match request.provider {
        PrivilegeProvider::None => {
            bail!("automatic privilege escalation is disabled by configuration")
        }
        PrivilegeProvider::Auto => resolve_first_available(PrivilegeProvider::Auto, search_path)
            .ok_or_else(|| {
                anyhow!(
                    "failed to locate a supported privilege provider on PATH; install one of `doas`, `sudo`, `run0`, or `su`, or set `[privilege].provider` explicitly"
                )
            }),
        requested => {
            if let Some(provider) = resolve_specific(requested, search_path) {
                return Ok(provider);
            }

            resolve_first_available(requested, search_path).ok_or_else(|| {
                anyhow!(
                    "failed to locate privilege provider `{}` on PATH, and no supported fallback provider was found",
                    provider_label(requested)
                )
            })
        }
    }
}

fn resolve_specific(
    requested: PrivilegeProvider,
    search_path: Option<&OsStr>,
) -> Option<ResolvedProvider> {
    let binary_name = provider_binary_name(requested)?;
    let binary_path = lookup_binary(binary_name, search_path)?;

    Some(ResolvedProvider {
        requested,
        effective: requested,
        binary_name,
        binary_path,
    })
}

fn resolve_first_available(
    requested: PrivilegeProvider,
    search_path: Option<&OsStr>,
) -> Option<ResolvedProvider> {
    AUTO_PROVIDER_ORDER
        .iter()
        .find_map(|(provider, binary_name)| {
            lookup_binary(binary_name, search_path).map(|binary_path| ResolvedProvider {
                requested,
                effective: *provider,
                binary_name,
                binary_path,
            })
        })
}

fn provider_binary_name(provider: PrivilegeProvider) -> Option<&'static str> {
    match provider {
        PrivilegeProvider::Auto | PrivilegeProvider::None => None,
        PrivilegeProvider::Doas => Some("doas"),
        PrivilegeProvider::Sudo => Some("sudo"),
        PrivilegeProvider::Run0 => Some("run0"),
        PrivilegeProvider::Su => Some("su"),
    }
}

pub(super) fn provider_label(provider: PrivilegeProvider) -> &'static str {
    match provider {
        PrivilegeProvider::Auto => "auto",
        PrivilegeProvider::Doas => "doas",
        PrivilegeProvider::Sudo => "sudo",
        PrivilegeProvider::Run0 => "run0",
        PrivilegeProvider::Su => "su",
        PrivilegeProvider::None => "none",
    }
}

fn lookup_binary(binary_name: &str, search_path: Option<&OsStr>) -> Option<PathBuf> {
    let binary_path = Path::new(binary_name);
    if binary_path.components().count() > 1 {
        return is_executable(binary_path).then(|| binary_path.to_path_buf());
    }

    let search_path = search_path?;
    env::split_paths(search_path).find_map(|directory| {
        let candidate = directory.join(binary_name);
        is_executable(&candidate).then_some(candidate)
    })
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && (metadata.permissions().mode() & 0o111 != 0))
        .unwrap_or(false)
}
