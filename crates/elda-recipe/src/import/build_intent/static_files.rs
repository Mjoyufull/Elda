use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::{BuildIntent, clean_bin_name, read_toml, sorted_unique};

pub(super) fn go_intent(source_dir: &Path, recipe_name: &str) -> Option<BuildIntent> {
    if !source_dir.join("go.mod").is_file() {
        return None;
    }

    let mut bins = Vec::new();
    let cmd_dir = source_dir.join("cmd");
    if let Ok(entries) = fs::read_dir(cmd_dir) {
        bins.extend(entries.flatten().filter_map(|entry| {
            let path = entry.path();
            (path.is_dir() && path.join("main.go").is_file())
                .then(|| clean_bin_name(entry.file_name().to_str()))
                .flatten()
        }));
    }
    if bins.is_empty() && source_dir.join("main.go").is_file() {
        bins.push(recipe_name.to_owned());
    }

    Some(BuildIntent {
        system: "go".to_owned(),
        bins: sorted_unique(bins),
    })
}

pub(super) fn meson_intent(source_dir: &Path) -> Option<BuildIntent> {
    let contents = fs::read_to_string(source_dir.join("meson.build")).ok()?;
    Some(BuildIntent {
        system: "meson".to_owned(),
        bins: sorted_unique(call_names(&contents, "executable")),
    })
}

pub(super) fn cmake_intent(source_dir: &Path) -> Option<BuildIntent> {
    let contents = fs::read_to_string(source_dir.join("CMakeLists.txt")).ok()?;
    Some(BuildIntent {
        system: "cmake".to_owned(),
        bins: sorted_unique(call_names(&contents, "add_executable")),
    })
}

pub(super) fn python_intent(source_dir: &Path) -> Option<BuildIntent> {
    let manifest = source_dir.join("pyproject.toml");
    if !manifest.is_file() && !source_dir.join("setup.py").is_file() {
        return None;
    }

    let bins = read_toml::<PythonProject>(&manifest)
        .ok()
        .map(|project| {
            project
                .project
                .scripts
                .into_keys()
                .chain(project.tool.poetry.scripts.into_keys())
                .filter_map(|name| clean_bin_name(Some(&name)))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(BuildIntent {
        system: "python".to_owned(),
        bins: sorted_unique(bins),
    })
}

pub(super) fn nimble_intent(source_dir: &Path, recipe_name: &str) -> Option<BuildIntent> {
    let path = fs::read_dir(source_dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "nimble"))?;
    let contents = fs::read_to_string(path).ok()?;
    let bins = nimble_bins(&contents);
    Some(BuildIntent {
        system: "nimble".to_owned(),
        bins: if bins.is_empty() {
            vec![recipe_name.to_owned()]
        } else {
            bins
        },
    })
}

pub(super) fn zig_intent(source_dir: &Path) -> Option<BuildIntent> {
    let contents = fs::read_to_string(source_dir.join("build.zig")).ok()?;
    Some(BuildIntent {
        system: "zig".to_owned(),
        bins: sorted_unique(zig_executable_names(&contents)),
    })
}

pub(super) fn make_intent(source_dir: &Path) -> Option<BuildIntent> {
    (source_dir.join("Makefile").is_file() || source_dir.join("makefile").is_file()).then(|| {
        BuildIntent {
            system: "make".to_owned(),
            bins: Vec::new(),
        }
    })
}

fn call_names(contents: &str, function: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| line.split_once(function))
        .filter_map(|(_, rest)| rest.split_once('(').map(|(_, args)| args.trim_start()))
        .filter_map(first_call_arg)
        .collect()
}

fn first_call_arg(args: &str) -> Option<String> {
    let quote = args.chars().next().filter(|ch| *ch == '\'' || *ch == '"')?;
    let end = args[1..].find(quote)?;
    clean_bin_name(Some(&args[1..1 + end]))
}

fn nimble_bins(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| line.trim().strip_prefix("bin"))
        .filter_map(|line| line.trim_start().strip_prefix('=').map(str::trim))
        .flat_map(quoted_values)
        .filter_map(|name| clean_bin_name(Some(&name)))
        .collect()
}

fn zig_executable_names(contents: &str) -> Vec<String> {
    contents
        .split("addExecutable")
        .skip(1)
        .filter_map(|chunk| chunk.split(".name").nth(1))
        .filter_map(|chunk| chunk.split('=').nth(1))
        .flat_map(quoted_values)
        .filter_map(|name| clean_bin_name(Some(&name)))
        .collect()
}

fn quoted_values(value: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find(['\'', '"']) {
        let quote = rest.as_bytes()[start] as char;
        let after = &rest[start + 1..];
        let Some(end) = after.find(quote) else {
            break;
        };
        values.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    values
}

#[derive(Default, Deserialize)]
struct PythonProject {
    #[serde(default)]
    project: PythonProjectTable,
    #[serde(default)]
    tool: PythonToolTable,
}

#[derive(Default, Deserialize)]
struct PythonProjectTable {
    #[serde(default)]
    scripts: std::collections::BTreeMap<String, toml::Value>,
}

#[derive(Default, Deserialize)]
struct PythonToolTable {
    #[serde(default)]
    poetry: PythonPoetryTable,
}

#[derive(Default, Deserialize)]
struct PythonPoetryTable {
    #[serde(default)]
    scripts: std::collections::BTreeMap<String, toml::Value>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::python_intent;

    #[test]
    fn python_reads_project_scripts() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::write(
            tempdir.path().join("pyproject.toml"),
            "[project.scripts]\ndemo = \"demo:main\"\n",
        )
        .expect("pyproject should exist");

        let intent = python_intent(tempdir.path()).expect("python intent should parse");

        assert_eq!(intent.system, "python");
        assert_eq!(intent.bins, ["demo"]);
    }
}
