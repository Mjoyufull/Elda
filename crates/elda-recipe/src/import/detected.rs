use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct DetectedStrategies {
    pub(super) elda_native: bool,
    pub(super) nix_flake: bool,
    pub(super) gentoo_package: Option<String>,
    pub(super) aur_pkgbuild: bool,
    pub(super) xbps_template: bool,
    pub(super) native_builds: Vec<String>,
}

impl DetectedStrategies {
    pub(super) fn read(source_dir: &Path) -> Self {
        Self {
            elda_native: source_dir.join("pkg.lua").is_file()
                || source_dir.join("packages").is_dir(),
            nix_flake: source_dir.join("flake.nix").is_file(),
            gentoo_package: detect_gentoo_package(source_dir),
            aur_pkgbuild: source_dir.join("PKGBUILD").is_file(),
            xbps_template: source_dir.join("template").is_file(),
            native_builds: native_build_files(source_dir),
        }
    }
}

fn native_build_files(source_dir: &Path) -> Vec<String> {
    [
        ("cargo", "Cargo.toml"),
        ("cmake", "CMakeLists.txt"),
        ("go", "go.mod"),
        ("make", "Makefile"),
        ("make", "makefile"),
        ("meson", "meson.build"),
        ("python", "pyproject.toml"),
        ("python", "setup.py"),
        ("zig", "build.zig"),
        ("nimble", "*.nimble"),
    ]
    .into_iter()
    .filter_map(|(strategy, marker)| has_build_marker(source_dir, marker).then_some(strategy))
    .map(str::to_owned)
    .collect::<std::collections::BTreeSet<_>>()
    .into_iter()
    .collect()
}

fn has_build_marker(source_dir: &Path, name: &str) -> bool {
    if name == "*.nimble" {
        return fs::read_dir(source_dir).is_ok_and(|entries| {
            entries.flatten().any(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "nimble")
            })
        });
    }

    source_dir.join(name).is_file()
}

fn detect_gentoo_package(source_dir: &Path) -> Option<String> {
    let mut packages = Vec::new();
    for category in fs::read_dir(source_dir).ok()? {
        let category = category.ok()?.path();
        if !category.is_dir() || !looks_like_gentoo_category(&category) {
            continue;
        }
        collect_ebuild_packages(source_dir, &category, &mut packages);
    }
    packages.sort();
    packages.dedup();

    match packages.as_slice() {
        [package] => Some(package.clone()),
        _ => None,
    }
}

fn collect_ebuild_packages(root: &Path, category: &Path, packages: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(category) else {
        return;
    };
    for entry in entries.flatten() {
        let package_dir = entry.path();
        if !package_dir.is_dir() || !contains_ebuild(&package_dir) {
            continue;
        }
        if let Ok(relative) = package_dir.strip_prefix(root) {
            packages.push(path_to_package(relative));
        }
    }
}

fn looks_like_gentoo_category(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.contains('-'))
}

fn contains_ebuild(path: &Path) -> bool {
    fs::read_dir(path).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "ebuild")
        })
    })
}

fn path_to_package(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
