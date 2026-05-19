use serde_json::Value;

use crate::CommandReport;
use crate::app_render_support::{json_string, json_u64, render_header, render_section};

pub(crate) fn render_git_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    if details.get("git_releases").is_some() {
        return render_git_releases_report(report);
    }
    render_git_tags_report(report)
}

fn render_git_tags_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let git_tags = details.get("git_tags")?;
    let target = json_string(git_tags, &["target"]).unwrap_or("<unknown>");
    let tags = git_tags.get("tags")?.as_array()?;
    let mut lines = vec![format!("target: {target}"), format!("tags: {}", tags.len())];
    let release_join = details.get("release_join");
    for tag in tags {
        let name = json_string(tag, &["tag"]).unwrap_or("<unknown>");
        let object = json_string(tag, &["object"]).unwrap_or("<unknown>");
        let version = json_string(tag, &["normalized_version"]).unwrap_or("unparsed");
        let confidence = json_string(tag, &["version_confidence"]).unwrap_or("raw");
        let release = release_join
            .and_then(|join| release_join_line(join, name))
            .unwrap_or_default();
        lines.push(format!(
            "{name}: {version} [{confidence}] {object}{release}"
        ));
    }

    Some(format!(
        "{}\n{}\n\n{}",
        render_header(report.area, report.status),
        report.summary,
        render_section("Git Tags", &lines),
    ))
}

fn release_join_line(join: &Value, tag: &str) -> Option<String> {
    let joined = join.get("joined_tags")?.as_array()?;
    let entry = joined
        .iter()
        .find(|entry| json_string(entry, &["tag"]) == Some(tag))?;
    let has_release = entry.get("has_release")?.as_bool()?;
    if !has_release {
        return Some(" release=none".to_owned());
    }
    let asset_count = json_u64(entry, &["asset_count"]).unwrap_or(0);
    let recommended = json_string(entry, &["recommended_asset"]).unwrap_or("none");
    Some(format!(
        " release=yes assets={asset_count} recommended={recommended}"
    ))
}

fn render_git_releases_report(report: &CommandReport) -> Option<String> {
    let details = report.details.as_ref()?;
    let git_releases = details.get("git_releases")?;
    let repo = json_string(git_releases, &["repo"]).unwrap_or("<unknown>");
    let releases = git_releases.get("releases")?.as_array()?;
    let mut lines = vec![
        format!("repo: {repo}"),
        format!("releases: {}", releases.len()),
    ];
    for release in releases {
        append_release_lines(&mut lines, release);
    }

    Some(format!(
        "{}\n{}\n\n{}",
        render_header(report.area, report.status),
        report.summary,
        render_section("Git Releases", &lines),
    ))
}

fn append_release_lines(lines: &mut Vec<String>, release: &Value) {
    let tag = json_string(release, &["tag"]).unwrap_or("<unknown>");
    let version = json_string(release, &["normalized_version"]).unwrap_or("unparsed");
    let confidence = json_string(release, &["version_confidence"]).unwrap_or("raw");
    let recommended = json_string(release, &["recommended_asset"]).unwrap_or("none");
    lines.push(format!(
        "{tag}: {version} [{confidence}] recommended={recommended}"
    ));

    for asset in release
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        append_asset_line(lines, asset);
    }
}

fn append_asset_line(lines: &mut Vec<String>, asset: &Value) {
    let name = json_string(asset, &["name"]).unwrap_or("<unknown>");
    let kind = json_string(asset, &["kind"]).unwrap_or("unknown");
    let format = json_string(asset, &["format"]).unwrap_or("unknown");
    let compatibility = json_string(asset, &["compatibility"]).unwrap_or("unknown");
    let score = json_u64(asset, &["score"]).unwrap_or(0);
    lines.push(format!(
        "  asset {name}: {kind}/{format} {compatibility} score={score}"
    ));
}
