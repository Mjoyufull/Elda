use serde_json::Value;

pub(super) fn matching_signature(release: &Value, asset_name: &str) -> Option<String> {
    let assets = release.get("assets").and_then(Value::as_array)?;
    assets
        .iter()
        .find(|asset| is_signature_sidecar(asset, asset_name))
        .and_then(|asset| {
            asset
                .get("browser_download_url")
                .and_then(Value::as_str)
                .or_else(|| asset.get("url").and_then(Value::as_str))
                .or_else(|| asset.get("name").and_then(Value::as_str))
        })
        .map(ToOwned::to_owned)
}

pub(super) fn matching_sha256(release: &Value, asset_name: &str) -> Option<String> {
    let assets = release.get("assets").and_then(Value::as_array)?;
    if let Some(digest) = assets
        .iter()
        .find(|asset| asset.get("name").and_then(Value::as_str) == Some(asset_name))
        .and_then(asset_digest_sha256)
    {
        return Some(digest);
    }

    let sidecar = assets
        .iter()
        .find(|asset| is_checksum_sidecar(asset, asset_name))?;
    sidecar
        .get("sha256")
        .and_then(Value::as_str)
        .and_then(normalize_sha256)
        .or_else(|| fetch_sidecar_sha256(sidecar, asset_name))
}

fn asset_digest_sha256(asset: &Value) -> Option<String> {
    let digest = asset.get("digest").and_then(Value::as_str)?;
    digest.strip_prefix("sha256:").and_then(normalize_sha256)
}

fn is_signature_sidecar(asset: &Value, asset_name: &str) -> bool {
    let Some(name) = asset.get("name").and_then(Value::as_str) else {
        return false;
    };
    [
        format!("{asset_name}.sig"),
        format!("{asset_name}.asc"),
        format!("{asset_name}.minisig"),
        format!("{asset_name}.sign"),
    ]
    .iter()
    .any(|candidate| candidate == name)
}

fn is_checksum_sidecar(asset: &Value, asset_name: &str) -> bool {
    let Some(name) = asset.get("name").and_then(Value::as_str) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    let asset_lower = asset_name.to_ascii_lowercase();
    let per_asset = [
        format!("{asset_lower}.sha256"),
        format!("{asset_lower}.sha256sum"),
        format!("{asset_lower}.sha256.txt"),
        format!("{asset_lower}.sums"),
    ];
    if per_asset.iter().any(|candidate| candidate == &lower) {
        return true;
    }
    matches!(
        lower.as_str(),
        "sha256sums"
            | "sha256sums.txt"
            | "sha256sum.txt"
            | "checksums"
            | "checksums.txt"
            | "checksums.sha256"
            | "checksum.txt"
            | "sums.txt"
            | "release.sha256"
    )
}

fn fetch_sidecar_sha256(sidecar: &Value, asset_name: &str) -> Option<String> {
    let url = sidecar
        .get("browser_download_url")
        .and_then(Value::as_str)?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(2))
        .build();
    let body = agent
        .get(url)
        .set("User-Agent", "elda")
        .call()
        .ok()?
        .into_string()
        .ok()?;
    parse_checksum_text(&body, asset_name)
}

fn parse_checksum_text(contents: &str, asset_name: &str) -> Option<String> {
    let asset_lower = asset_name.to_ascii_lowercase();
    contents.lines().find_map(|line| {
        let trimmed = line.trim();
        let digest = trimmed.split_whitespace().next()?;
        if trimmed.split_whitespace().count() > 1
            && !trimmed.to_ascii_lowercase().contains(&asset_lower)
        {
            return None;
        }
        normalize_sha256(digest)
    })
}

fn normalize_sha256(value: &str) -> Option<String> {
    let value = value.trim();
    (value.len() == 64 && value.chars().all(|ch| ch.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}
