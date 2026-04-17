use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(in crate::tests) fn create_git_cargo_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(repo_dir.join("src")).expect("source dir should exist");
    fs::write(
        repo_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"{name}\"\npath = \"src/main.rs\"\n"
        ),
    )
    .expect("cargo manifest should be written");
    fs::write(
        repo_dir.join("src/main.rs"),
        "fn main() {\n    println!(\"sample-tool\");\n}\n",
    )
    .expect("main source should be written");

    make_git_repo(&repo_dir);
    repo_dir
}

pub(in crate::tests) fn create_git_make_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join("Makefile"),
        format!(
            "all:\n\tchmod +x {name}\n\ninstall:\n\tinstall -d $(DESTDIR)$(PREFIX)/bin\n\tinstall -m 0755 {name} $(DESTDIR)$(PREFIX)/bin/{name}\n"
        ),
    )
    .expect("makefile should be written");
    fs::write(repo_dir.join(name), "#!/bin/sh\necho make tool\n")
        .expect("make binary should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_git_cmake_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join("CMakeLists.txt"),
        format!(
            "cmake_minimum_required(VERSION 3.16)\nproject({name} C)\nadd_executable({name} main.c)\ninstall(TARGETS {name} RUNTIME DESTINATION bin)\n"
        ),
    )
    .expect("cmake file should be written");
    fs::write(
        repo_dir.join("main.c"),
        "#include <stdio.h>\nint main(void) { puts(\"cmake tool\"); return 0; }\n",
    )
    .expect("c source should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_git_meson_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join("meson.build"),
        format!("project('{name}', 'c')\nexecutable('{name}', 'main.c', install : true)\n"),
    )
    .expect("meson file should be written");
    fs::write(
        repo_dir.join("main.c"),
        "#include <stdio.h>\nint main(void) { puts(\"meson tool\"); return 0; }\n",
    )
    .expect("c source should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_git_go_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join("go.mod"),
        format!("module example.com/{name}\n\ngo 1.22\n"),
    )
    .expect("go.mod should be written");
    fs::write(
        repo_dir.join("main.go"),
        "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"go tool\")\n}\n",
    )
    .expect("go source should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_git_zig_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(repo_dir.join("src")).expect("repo dir should exist");
    fs::write(
        repo_dir.join("build.zig"),
        format!(
            "const std = @import(\"std\");\n\npub fn build(b: *std.Build) void {{\n    const target = b.standardTargetOptions(.{{}});\n    const optimize = b.standardOptimizeOption(.{{}});\n    const exe = b.addExecutable(.{{\n        .name = \"{name}\",\n        .root_module = b.createModule(.{{\n            .root_source_file = b.path(\"src/main.zig\"),\n            .target = target,\n            .optimize = optimize,\n        }}),\n    }});\n    b.installArtifact(exe);\n}}\n"
        ),
    )
    .expect("build.zig should be written");
    fs::write(
        repo_dir.join("src/main.zig"),
        "const std = @import(\"std\");\n\npub fn main() !void {\n    const stdout = std.fs.File.stdout();\n    try stdout.writeAll(\"zig tool\\n\");\n}\n",
    )
    .expect("zig source should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_git_python_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    let module_name = name.replace('-', "_");
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join("setup.py"),
        format!(
            "from setuptools import setup\n\nsetup(\n    name=\"{name}\",\n    version=\"0.1.0\",\n    py_modules=[\"{module_name}\"],\n    entry_points={{\"console_scripts\": [\"{name}={module_name}:main\"]}},\n)\n"
        ),
    )
    .expect("setup.py should be written");
    fs::write(
        repo_dir.join(format!("{module_name}.py")),
        "def main():\n    print(\"python tool\")\n",
    )
    .expect("python module should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_git_nimble_repo(root: &Path, name: &str) -> PathBuf {
    let repo_dir = root.join(name);
    fs::create_dir_all(&repo_dir).expect("repo dir should exist");
    fs::write(
        repo_dir.join(format!("{name}.nimble")),
        format!(
            "version       = \"0.1.0\"\nauthor        = \"Elda\"\ndescription   = \"Test project\"\nlicense       = \"MIT\"\nbin           = @[\"{name}\"]\nsrcDir        = \".\"\n"
        ),
    )
    .expect("nimble file should be written");
    fs::write(repo_dir.join(format!("{name}.nim")), "echo \"nim tool\"\n")
        .expect("nim source should be written");
    make_git_repo(&repo_dir);

    repo_dir
}

pub(in crate::tests) fn create_vendor_binary(root: &Path, name: &str) -> PathBuf {
    let binary_path = root.join(format!("{name}-vendor"));
    fs::write(&binary_path, "#!/bin/sh\necho binary lane\n")
        .expect("vendor binary should be written");
    make_executable(&binary_path);

    binary_path
}

pub(in crate::tests) fn create_script_binary(root: &Path, name: &str, output: &str) -> PathBuf {
    let binary_path = root.join(name);
    fs::write(&binary_path, format!("#!/bin/sh\necho '{output}'\n"))
        .expect("script binary should be written");
    make_executable(&binary_path);

    binary_path
}

pub(in crate::tests) fn make_executable(path: &Path) {
    let mut permissions = fs::metadata(path)
        .expect("binary metadata should exist")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("binary permissions should be set");
    }
}

pub(in crate::tests) fn run_git(repo_dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo_dir)
        .args(args)
        .status()
        .expect("git command should launch");
    assert!(status.success(), "git command failed: {:?}", args);
}

pub(in crate::tests) fn make_git_repo(repo_dir: &Path) {
    run_git(repo_dir, &["init", "-b", "main"]);
    run_git(repo_dir, &["config", "user.email", "elda@example.invalid"]);
    run_git(repo_dir, &["config", "user.name", "Elda Tests"]);
    run_git(repo_dir, &["add", "."]);
    run_git(repo_dir, &["commit", "-m", "initial"]);
}

pub(in crate::tests) fn all_tools_available(tools: &[&str]) -> bool {
    tools.iter().all(|tool| {
        Command::new("sh")
            .arg("-lc")
            .arg(format!("command -v {tool} >/dev/null 2>&1"))
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    })
}
