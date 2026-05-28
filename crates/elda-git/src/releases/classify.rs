use std::env::consts;

use super::github::ReleaseAssetResponse;
use super::{AssetCompatibility, AssetFormat, AssetKind, GitReleaseAssetEntry};

pub(crate) fn classify_release_asset(asset: ReleaseAssetResponse) -> GitReleaseAssetEntry {
    let name = asset.name;
    let lower = name.to_ascii_lowercase();
    let format = asset_format(&lower);
    let kind = asset_kind(&lower, format);
    let os = detect_alias(&lower, os_aliases()).map(str::to_owned);
    let arch = detect_alias(&lower, arch_aliases()).map(str::to_owned);
    let libc = detect_alias(&lower, libc_aliases()).map(str::to_owned);
    let compatibility = asset_compatibility(kind, os.as_deref(), arch.as_deref(), libc.as_deref());
    let score = asset_score(kind, compatibility, libc.as_deref());

    GitReleaseAssetEntry {
        name,
        url: asset.browser_download_url,
        kind,
        format,
        os,
        arch,
        libc,
        compatibility,
        score,
    }
}

fn asset_kind(lower: &str, format: AssetFormat) -> AssetKind {
    if matches!(format, AssetFormat::Checksum) {
        AssetKind::Checksum
    } else if matches!(format, AssetFormat::Signature) {
        AssetKind::Signature
    } else if lower.ends_with(".json") || lower.ends_with(".spdx") || lower.ends_with(".sbom") {
        AssetKind::Metadata
    } else {
        AssetKind::Payload
    }
}

fn asset_format(lower: &str) -> AssetFormat {
    if checksum_suffix(lower) {
        AssetFormat::Checksum
    } else if signature_suffix(lower) {
        AssetFormat::Signature
    } else {
        payload_format(lower)
    }
}

fn payload_format(lower: &str) -> AssetFormat {
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        AssetFormat::TarGz
    } else if lower.ends_with(".tar.xz") || lower.ends_with(".txz") {
        AssetFormat::TarXz
    } else if lower.ends_with(".tar.zst") || lower.ends_with(".tzst") {
        AssetFormat::TarZst
    } else if lower.ends_with(".zip") {
        AssetFormat::Zip
    } else if lower.ends_with(".appimage") {
        AssetFormat::AppImage
    } else if lower.ends_with(".deb") {
        AssetFormat::Deb
    } else if lower.ends_with(".rpm") {
        AssetFormat::Rpm
    } else if lower.ends_with(".apk") {
        AssetFormat::Apk
    } else if lower.ends_with(".pkg.tar.zst") || lower.ends_with(".pkg.tar.xz") {
        AssetFormat::PacmanPackage
    } else if has_no_extension(lower) {
        AssetFormat::RawBinary
    } else {
        AssetFormat::Unknown
    }
}

fn asset_compatibility(
    kind: AssetKind,
    os: Option<&str>,
    arch: Option<&str>,
    libc: Option<&str>,
) -> AssetCompatibility {
    if kind != AssetKind::Payload {
        return AssetCompatibility::Sidecar;
    }
    match (os.is_some(), arch.is_some(), libc.is_some()) {
        (true, true, true) => AssetCompatibility::NativeExact,
        (true, true, false) => AssetCompatibility::NativePartial,
        (false, false, false) => AssetCompatibility::Unknown,
        _ => AssetCompatibility::Foreign,
    }
}

fn asset_score(kind: AssetKind, compatibility: AssetCompatibility, libc: Option<&str>) -> u8 {
    if kind != AssetKind::Payload {
        return 0;
    }
    let base_score = match compatibility {
        AssetCompatibility::NativeExact => 30,
        AssetCompatibility::NativePartial if libc_aliases().is_empty() => 25,
        AssetCompatibility::NativePartial => 20,
        AssetCompatibility::Unknown => 5,
        AssetCompatibility::Foreign | AssetCompatibility::Sidecar => 0,
    };
    base_score + u8::from(libc.is_some())
}

fn detect_alias<'a>(lower: &str, aliases: &'a [&'a str]) -> Option<&'a str> {
    aliases.iter().copied().find(|alias| lower.contains(alias))
}

fn checksum_suffix(lower: &str) -> bool {
    [".sha256", ".sha256sum", ".sha512", ".sha512sum", ".sum"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn signature_suffix(lower: &str) -> bool {
    [".sig", ".asc", ".minisig", ".sign"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn has_no_extension(lower: &str) -> bool {
    !lower.rsplit('/').next().unwrap_or(lower).contains('.')
}

fn os_aliases() -> &'static [&'static str] {
    match consts::OS {
        "linux" => &["linux", "unknown-linux"],
        "macos" => &["darwin", "apple-darwin", "macos", "osx"],
        "freebsd" => &["freebsd", "unknown-freebsd"],
        "windows" => &["windows", "pc-windows", "win64", "win32"],
        _ => &[consts::OS],
    }
}

fn arch_aliases() -> &'static [&'static str] {
    match consts::ARCH {
        "x86_64" => &["x86_64", "amd64"],
        "aarch64" => &["aarch64", "arm64"],
        "x86" => &["i686", "i386", "386"],
        "arm" => &["armv7", "armv7l", "armhf"],
        "riscv64" => &["riscv64"],
        "powerpc64" => &["ppc64le", "powerpc64le"],
        _ => &[consts::ARCH],
    }
}

fn libc_aliases() -> &'static [&'static str] {
    if cfg!(target_env = "gnu") {
        &["gnu", "glibc"]
    } else if cfg!(target_env = "musl") {
        &["musl"]
    } else if cfg!(target_env = "msvc") {
        &["msvc"]
    } else {
        &[]
    }
}
