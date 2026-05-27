use std::path::Path;
use std::process::Command;

pub fn emit_build_metadata(workspace: &Path) {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=ELDA_BUILD_PROFILE={profile}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=ELDA_BUILD_TARGET={target}");

    println!("cargo:rustc-env=ELDA_BUILD_DATE={}", build_date(workspace));
    println!("cargo:rustc-env=ELDA_GIT_COMMIT={}", git_commit(workspace));
}

fn build_date(workspace: &Path) -> String {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .map(|epoch| format!("reproducible epoch {epoch}"))
        .or_else(|| git_output(workspace, &["show", "-s", "--format=%cI", "HEAD"]))
        .unwrap_or_default()
}

fn git_commit(workspace: &Path) -> String {
    git_output(workspace, &["rev-parse", "--short=12", "HEAD"]).unwrap_or_default()
}

fn git_output(workspace: &Path, args: &[&str]) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}
