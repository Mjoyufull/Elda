use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::metadata::GeneratedMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SrcinfoBinaryArchive {
    pub(super) url: String,
    pub(super) sha256: String,
    pub(super) binary: String,
}

pub(super) fn read_srcinfo_metadata(source_dir: &Path) -> Option<GeneratedMetadata> {
    let contents = fs::read_to_string(source_dir.join(".SRCINFO")).ok()?;
    let values = parse_srcinfo(&contents);

    Some(GeneratedMetadata {
        description: first(&values, "pkgdesc"),
        licenses: all(&values, "license"),
        upstream: first(&values, "url"),
        version: first(&values, "pkgver"),
        rel: first(&values, "pkgrel").and_then(|value| value.parse().ok()),
        depends: all(&values, "depends"),
        makedepends: all(&values, "makedepends"),
        checkdepends: all(&values, "checkdepends"),
        provides: all(&values, "provides"),
        conflicts: all(&values, "conflicts"),
        replaces: all(&values, "replaces"),
    })
}

pub(super) fn read_srcinfo_binary_archive(source_dir: &Path) -> Option<SrcinfoBinaryArchive> {
    let contents = fs::read_to_string(source_dir.join(".SRCINFO")).ok()?;
    let values = parse_srcinfo(&contents);
    if !is_binary_package(&values) {
        return None;
    }
    let arch = srcinfo_native_arch();
    let source = first(&values, &format!("source_{arch}")).or_else(|| first(&values, "source"))?;
    let sha256 =
        first(&values, &format!("sha256sums_{arch}")).or_else(|| first(&values, "sha256sums"))?;
    let url = source
        .rsplit_once("::")
        .map_or(source.as_str(), |(_, url)| url)
        .to_owned();
    if sha256 == "SKIP" || !supported_archive_url(&url) {
        return None;
    }

    Some(SrcinfoBinaryArchive {
        url,
        sha256,
        binary: inferred_binary_name(&values)?,
    })
}

fn is_binary_package(values: &HashMap<String, Vec<String>>) -> bool {
    first(values, "pkgname")
        .or_else(|| first(values, "pkgbase"))
        .is_some_and(|name| name.ends_with("-bin"))
}

fn parse_srcinfo(contents: &str) -> HashMap<String, Vec<String>> {
    let mut values = HashMap::<String, Vec<String>>::new();
    for line in contents.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key.is_empty() || value.is_empty() {
            continue;
        }
        let entries = values.entry(key.to_owned()).or_default();
        if !entries.iter().any(|entry| entry == value) {
            entries.push(value.to_owned());
        }
    }
    values
}

fn first(values: &HashMap<String, Vec<String>>, key: &str) -> Option<String> {
    values.get(key).and_then(|entries| entries.first()).cloned()
}

fn all(values: &HashMap<String, Vec<String>>, key: &str) -> Vec<String> {
    values.get(key).cloned().unwrap_or_default()
}

fn srcinfo_native_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        other => other,
    }
}

fn supported_archive_url(url: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    [
        ".tar", ".tar.gz", ".tgz", ".tar.zst", ".tzst", ".tar.xz", ".txz",
    ]
    .iter()
    .any(|suffix| path.ends_with(suffix))
}

fn inferred_binary_name(values: &HashMap<String, Vec<String>>) -> Option<String> {
    all(values, "provides")
        .into_iter()
        .map(|value| {
            value
                .split(['<', '=', '>'])
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .find(|value| !value.is_empty())
        .or_else(|| first(values, "pkgname"))
        .or_else(|| first(values, "pkgbase"))
        .map(|value| value.strip_suffix("-bin").unwrap_or(&value).to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{parse_srcinfo, read_srcinfo_binary_archive};

    #[test]
    fn srcinfo_values_are_collected_without_duplicates() {
        let values = parse_srcinfo(
            "pkgbase = demo\n\tpkgdesc = Demo package\n\tdepends = glibc\n\tdepends = glibc\n",
        );

        assert_eq!(values["pkgdesc"], ["Demo package"]);
        assert_eq!(values["depends"], ["glibc"]);
    }

    #[test]
    fn srcinfo_binary_archive_uses_native_arch_source_and_provided_launcher() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::write(
            tempdir.path().join(".SRCINFO"),
            "pkgbase = demo-bin\n\tpkgname = demo-bin\n\tprovides = demo=1.0\n\tsource_x86_64 = demo.tar.xz::https://example.invalid/demo-x86_64.tar.xz\n\tsha256sums_x86_64 = aaaa\n",
        )
        .expect(".SRCINFO should exist");

        let archive = read_srcinfo_binary_archive(tempdir.path()).expect("archive should parse");

        assert_eq!(archive.url, "https://example.invalid/demo-x86_64.tar.xz");
        assert_eq!(archive.sha256, "aaaa");
        assert_eq!(archive.binary, "demo");
    }

    #[test]
    fn srcinfo_source_archive_is_not_treated_as_prebuilt_binary() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::write(
            tempdir.path().join(".SRCINFO"),
            "pkgbase = demo\n\tpkgname = demo\n\tprovides = demo\n\tsource_x86_64 = demo.tar.xz::https://example.invalid/demo-x86_64.tar.xz\n\tsha256sums_x86_64 = aaaa\n",
        )
        .expect(".SRCINFO should exist");

        assert!(read_srcinfo_binary_archive(tempdir.path()).is_none());
    }
}
