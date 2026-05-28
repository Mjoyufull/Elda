use std::fs;
use std::path::{Path, PathBuf};

use rowan::ast::AstNode;

use super::strategy::SourceStrategy;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct GeneratedMetadata {
    pub description: Option<String>,
    pub licenses: Vec<String>,
    pub upstream: Option<String>,
    pub version: Option<String>,
    pub rel: Option<u64>,
    pub depends: Vec<String>,
    pub makedepends: Vec<String>,
    pub checkdepends: Vec<String>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
}

pub(super) fn read_generated_metadata(
    source_dir: Option<&Path>,
    strategy: &SourceStrategy,
) -> GeneratedMetadata {
    let Some(source_dir) = source_dir else {
        return GeneratedMetadata::default();
    };
    match strategy {
        SourceStrategy::GentooEbuild { package } => gentoo_metadata(source_dir, package),
        SourceStrategy::AurPkgbuild => aur_metadata(&source_dir.join("PKGBUILD")),
        SourceStrategy::XbpsTemplate { package } => {
            let path = if source_dir.join(package).join("template").is_file() {
                source_dir.join(package).join("template")
            } else {
                source_dir.join("template")
            };
            xbps_metadata(&path)
        }
        SourceStrategy::NixFlake => nix_metadata(&source_dir.join("flake.nix")),
        SourceStrategy::EldaNative { .. }
        | SourceStrategy::Git
        | SourceStrategy::GithubRelease(_) => GeneratedMetadata::default(),
    }
}

fn aur_metadata(path: &Path) -> GeneratedMetadata {
    let contents = read_file(path);
    GeneratedMetadata {
        description: assignment_value(&contents, "pkgdesc"),
        licenses: array_values(&contents, "license"),
        upstream: assignment_value(&contents, "url"),
        version: assignment_value(&contents, "pkgver"),
        rel: assignment_value(&contents, "pkgrel").and_then(|value| value.parse().ok()),
        depends: array_values(&contents, "depends"),
        makedepends: array_values(&contents, "makedepends"),
        checkdepends: array_values(&contents, "checkdepends"),
        provides: array_values(&contents, "provides"),
        conflicts: array_values(&contents, "conflicts"),
        replaces: array_values(&contents, "replaces"),
    }
}

fn xbps_metadata(path: &Path) -> GeneratedMetadata {
    let contents = read_file(path);
    GeneratedMetadata {
        description: assignment_value(&contents, "short_desc"),
        licenses: split_words(assignment_value(&contents, "license").as_deref()),
        upstream: assignment_value(&contents, "homepage"),
        version: assignment_value(&contents, "version"),
        rel: assignment_value(&contents, "revision").and_then(|value| value.parse().ok()),
        depends: split_words(assignment_value(&contents, "depends").as_deref()),
        makedepends: split_words(assignment_value(&contents, "makedepends").as_deref()),
        checkdepends: split_words(assignment_value(&contents, "checkdepends").as_deref()),
        provides: split_words(assignment_value(&contents, "provides").as_deref()),
        conflicts: split_words(assignment_value(&contents, "conflicts").as_deref()),
        replaces: Vec::new(),
    }
}

fn gentoo_metadata(root: &Path, package: &str) -> GeneratedMetadata {
    // Try the joined path first (overlay root + category/package)
    // Then try the package name alone (in case root is the category dir)
    // Then try root itself (in case root is the package dir)
    let pkg_name = package.rsplit_once('/').map(|(_, p)| p).unwrap_or(package);
    let ebuild = first_ebuild(root, package)
        .or_else(|| first_ebuild(root, pkg_name))
        .or_else(|| first_ebuild(root, ""));

    let Some(ebuild) = ebuild else {
        return GeneratedMetadata::default();
    };
    let contents = read_file(&ebuild);
    GeneratedMetadata {
        description: assignment_value(&contents, "DESCRIPTION"),
        licenses: split_words(assignment_value(&contents, "LICENSE").as_deref()),
        upstream: assignment_value(&contents, "HOMEPAGE"),
        version: gentoo_version(&ebuild),
        rel: Some(1),
        depends: split_words(assignment_value(&contents, "RDEPEND").as_deref()),
        makedepends: split_words(assignment_value(&contents, "BDEPEND").as_deref()),
        checkdepends: Vec::new(),
        provides: Vec::new(),
        conflicts: Vec::new(),
        replaces: Vec::new(),
    }
}

fn nix_metadata(path: &Path) -> GeneratedMetadata {
    let contents = read_file(path);
    if contents.is_empty() {
        return GeneratedMetadata::default();
    }

    let root = rnix::Root::parse(&contents).syntax();

    let mut description = None;
    let mut meta_description = None;
    let mut homepage = None;
    let mut license = None;
    let mut pname = None;
    let mut version = None;

    for desc in root.descendants() {
        if desc.kind() != rnix::SyntaxKind::NODE_ATTRPATH_VALUE {
            continue;
        }
        let Some(attrpath_value) = rnix::ast::AttrpathValue::cast(desc.clone()) else {
            continue;
        };
        let Some(path_node) = attrpath_value.attrpath() else {
            continue;
        };

        let attrs: Vec<String> = path_node
            .attrs()
            .filter_map(|attr| {
                if let rnix::ast::Attr::Ident(ident) = attr {
                    ident.ident_token().map(|t| t.text().to_string())
                } else {
                    None
                }
            })
            .collect();

        if attrs.is_empty() {
            continue;
        }
        let Some(last) = attrs.last().map(String::as_str) else {
            continue;
        };

        // Determine if this node is inside a `meta` attribute set
        let in_meta = is_inside_meta_context(&desc) || attrs.iter().any(|a| a == "meta");

        if last == "description" {
            if let Some(val) = extract_nix_string(&attrpath_value) {
                if in_meta {
                    meta_description = Some(val.clone());
                }
                if description.is_none() {
                    description = Some(val);
                }
            }
        } else if (last == "homepage" || last == "downloadPage") && homepage.is_none() {
            homepage = extract_nix_string(&attrpath_value);
        } else if (last == "license" || last == "licenses") && license.is_none() {
            license = Some(extract_nix_raw_license(&attrpath_value));
        } else if last == "pname" && pname.is_none() {
            pname = extract_nix_string(&attrpath_value);
        } else if last == "version" && version.is_none() {
            version = extract_nix_string(&attrpath_value);
        }
    }

    let effective_description = meta_description.or(description);
    let licenses = license
        .map(|l| {
            l.split_whitespace()
                .map(|s| s.to_owned())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    GeneratedMetadata {
        description: effective_description,
        upstream: homepage,
        licenses,
        version,
        ..GeneratedMetadata::default()
    }
}

/// Check whether a syntax node is nested inside a `meta` attribute
/// context by walking ancestors.
fn is_inside_meta_context(node: &rnix::SyntaxNode) -> bool {
    use rowan::ast::AstNode;
    for parent in node.ancestors() {
        if parent.kind() == rnix::SyntaxKind::NODE_ATTRPATH_VALUE
            && let Some(parent_attr) = rnix::ast::AttrpathValue::cast(parent)
            && let Some(path) = parent_attr.attrpath()
        {
            for attr in path.attrs() {
                if let rnix::ast::Attr::Ident(ident) = attr
                    && let Some(token) = ident.ident_token()
                    && token.text() == "meta"
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract a string value from a Nix attribute assignment.
fn extract_nix_string(attrpath_value: &rnix::ast::AttrpathValue) -> Option<String> {
    use rowan::ast::AstNode;
    if let Some(val) = attrpath_value.value()
        && val.syntax().kind() == rnix::SyntaxKind::NODE_STRING
    {
        let text = val.syntax().text().to_string();
        let trimmed = text.trim_matches('"').to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    None
}

/// Extract a raw license text, stripping common Nix license prefixes.
fn extract_nix_raw_license(attrpath_value: &rnix::ast::AttrpathValue) -> String {
    use rowan::ast::AstNode;
    if let Some(val) = attrpath_value.value() {
        let text = val.syntax().text().to_string();
        if text.starts_with('"') && text.ends_with('"') {
            return text.trim_matches('"').to_string();
        }
        let stripped = text.strip_prefix("lib.licenses.").unwrap_or(&text);
        let stripped = stripped.strip_prefix("licenses.").unwrap_or(stripped);
        return stripped.trim().to_string();
    }
    String::new()
}

fn first_ebuild(root: &Path, package: &str) -> Option<PathBuf> {
    let mut entries = fs::read_dir(root.join(package))
        .ok()?
        .flatten()
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());
    entries.into_iter().map(|entry| entry.path()).find(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension == "ebuild")
    })
}

fn gentoo_version(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_string_lossy();
    stem.rsplit_once('-')
        .map(|(_, version)| version.to_owned())
        .filter(|version| !version.is_empty())
}

fn read_file(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn assignment_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return None;
        }
        let value = trimmed
            .strip_prefix(key)?
            .trim_start()
            .strip_prefix('=')?
            .trim();
        Some(clean_scalar(value))
    })
}

fn array_values(contents: &str, key: &str) -> Vec<String> {
    let Some(value) = assignment_value(contents, key) else {
        return Vec::new();
    };
    let inner = value
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(&value);
    split_words(Some(inner))
}

fn split_words(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split_whitespace()
        .map(clean_scalar)
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn clean_scalar(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim_matches('(')
        .trim_matches(')')
        .trim()
        .to_owned()
}
