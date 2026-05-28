use std::path::Path;
use std::process::Command;

pub fn emit(workspace: &Path) {
    println!("cargo:rerun-if-changed={}/.git/HEAD", workspace.display());
    println!("cargo:rerun-if-changed={}/.git/refs", workspace.display());
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=ELDA_BUILD_PROFILE={profile}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=ELDA_BUILD_TARGET={target}");

    let date = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .map(|epoch| format!("reproducible epoch {epoch}"))
        .or_else(|| git_head_date(workspace))
        .unwrap_or_default();
    println!("cargo:rustc-env=ELDA_BUILD_DATE={date}");

    let commit = git_short_commit(workspace).unwrap_or_default();
    println!("cargo:rustc-env=ELDA_GIT_COMMIT={commit}");
}

fn git_head_date(workspace: &Path) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["show", "-s", "--format=%cI", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn git_short_commit(workspace: &Path) -> Option<String> {
    Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
}
