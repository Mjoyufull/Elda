use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=ELDA_BUILD_PROFILE={profile}");

    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_owned());
    println!("cargo:rustc-env=ELDA_BUILD_TARGET={target}");

    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");

    let date = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .map(|epoch| format!("reproducible epoch {epoch}"))
        .or_else(|| {
            Command::new("git")
                .arg("-C")
                .arg(&workspace)
                .args(["show", "-s", "--format=%cI", "HEAD"])
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default();
    println!("cargo:rustc-env=ELDA_BUILD_DATE={date}");

    let commit = Command::new("git")
        .arg("-C")
        .arg(&workspace)
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    println!("cargo:rustc-env=ELDA_GIT_COMMIT={commit}");
}
