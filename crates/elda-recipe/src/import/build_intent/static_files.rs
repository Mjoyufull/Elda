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
    let path = sorted_nimble_paths(source_dir).ok()?.into_iter().next()?;
    let contents = fs::read_to_string(path).ok()?;
    let bins = nimble_bins(&contents);
    Some(BuildIntent {
        system: "nimble".to_owned(),
        bins: if bins.is_empty() {
            vec![recipe_name.to_owned()]
        } else {
            sorted_unique(bins)
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
    let args = args.trim_start();
    if let Some(quote) = args.chars().next().filter(|ch| *ch == '\'' || *ch == '"') {
        let end = args[1..].find(quote)?;
        return clean_bin_name(Some(&args[1..1 + end]));
    }

    let end = args
        .find(|ch: char| ch.is_whitespace() || ch == ',' || ch == ')')
        .unwrap_or(args.len());
    clean_bin_name(Some(&args[..end]))
}

fn sorted_nimble_paths(source_dir: &Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut paths = fs::read_dir(source_dir)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "nimble"))
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn nimble_bins(contents: &str) -> Vec<String> {
    bin_assignment_values(contents)
        .into_iter()
        .flat_map(|value| quoted_values(&value))
        .filter_map(|name| clean_bin_name(Some(&name)))
        .collect()
}

fn bin_assignment_values(contents: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut collecting = false;
    let mut current = String::new();

    for line in contents.lines().map(str::trim) {
        if collecting {
            current.push(' ');
            current.push_str(line);
            if line.contains(']') {
                values.push(std::mem::take(&mut current));
                collecting = false;
            }
            continue;
        }

        let Some(rest) = line.strip_prefix("bin") else {
            continue;
        };
        let Some(value) = rest.trim_start().strip_prefix('=').map(str::trim) else {
            continue;
        };
        if value.contains("@[") && !value.contains(']') {
            current.push_str(value);
            collecting = true;
        } else {
            values.push(value.to_owned());
        }
    }

    values
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

    use super::{cmake_intent, nimble_intent, python_intent};

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

    #[test]
    fn cmake_reads_unquoted_executable_targets() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::write(
            tempdir.path().join("CMakeLists.txt"),
            "add_executable(tool src/main.c)\nadd_executable(\"quoted-tool\" src/quoted.c)\n",
        )
        .expect("cmake file should exist");

        let intent = cmake_intent(tempdir.path()).expect("cmake intent should parse");

        assert_eq!(intent.bins, ["quoted-tool", "tool"]);
    }

    #[test]
    fn nimble_reads_multiline_bin_arrays() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::write(
            tempdir.path().join("demo.nimble"),
            "bin = @[\n  \"tool\",\n  \"helper\"\n]\n",
        )
        .expect("nimble file should exist");

        let intent = nimble_intent(tempdir.path(), "demo").expect("nimble intent should parse");

        assert_eq!(intent.bins, ["helper", "tool"]);
    }

    #[test]
    fn nimble_file_selection_is_sorted() {
        let tempdir = TempDir::new().expect("tempdir should exist");
        fs::write(tempdir.path().join("z.nimble"), "bin = @[\"z-tool\"]\n")
            .expect("nimble file should exist");
        fs::write(tempdir.path().join("a.nimble"), "bin = @[\"a-tool\"]\n")
            .expect("nimble file should exist");

        let intent = nimble_intent(tempdir.path(), "demo").expect("nimble intent should parse");

        assert_eq!(intent.bins, ["a-tool"]);
    }
}
