use std::env;

use serde_json::json;

use crate::app::AppContext;
use crate::config::SubmissionAuthKind;
use crate::error::CoreError;
use crate::{CommandReport, CommandRequest, ExitStatus};

use super::model::CiSubmissionRecord;
use super::store::{find_submission, load_submissions, save_submission};
use super::workspace::{CiWorkspacePaths, current_unix_timestamp, forge_pr_url, git_remote_url};

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostedReviewResult {
    url: String,
    kind: &'static str,
    review_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HostedForgeKind {
    GitHub,
    GitLab,
    Gitea,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostedForgeRepo {
    kind: HostedForgeKind,
    host: String,
    path: String,
    owner: String,
    repo: String,
}

#[derive(Debug, Clone)]
struct HostedReviewRequest<'a> {
    origin_url: &'a str,
    api_base_override: Option<&'a str>,
    branch_name: &'a str,
    target_branch: &'a str,
    title: &'a str,
    description: &'a str,
    token: &'a str,
}

impl AppContext {
    pub(super) fn handle_ci_pr(&self, request: CommandRequest) -> Result<CommandReport, CoreError> {
        let workspace = CiWorkspacePaths::new(self.database.layout());
        let submissions = load_submissions(&workspace)?;
        let selection = request.operands.first().cloned().unwrap_or_default();
        let mut submission = if selection.is_empty() {
            submissions
                .into_iter()
                .max_by_key(|record| record.updated_at)
        } else {
            find_submission(&submissions, &selection)
        }
        .ok_or_else(|| {
            CoreError::Operator("no ci submission is available for `ci pr`".to_owned())
        })?;
        let default_submission_target = self.config.submission.resolve_target();
        let remote_name = submission
            .remote_name
            .clone()
            .unwrap_or_else(|| default_submission_target.remote_name.clone());
        let submission_config = self.config.submission.resolve_for_remote(&remote_name);
        let target_branch = if submission.target_branch.trim().is_empty() {
            submission_config.base_branch.clone()
        } else {
            submission.target_branch.clone()
        };
        let origin_url = git_remote_url(&submission.packages_repo_path, &remote_name)?;
        let compare_url = origin_url
            .as_deref()
            .and_then(|url| forge_pr_url(url, &submission.branch_name, &target_branch));

        let created_review = if request.dry_run {
            None
        } else {
            maybe_create_hosted_review_from_config(
                &submission_config,
                &submission,
                origin_url.as_deref(),
                &target_branch,
            )?
        };

        if let Some(review) = created_review.as_ref() {
            submission.review_url = Some(review.url.clone());
            submission.review_kind = Some(review.kind.to_owned());
            submission.review_id = review.review_id.clone();
            submission.review_created_at = Some(current_unix_timestamp());
            submission.updated_at = current_unix_timestamp();
            save_submission(&workspace, &submission)?;
        }

        let pr_url = submission
            .review_url
            .clone()
            .or_else(|| created_review.as_ref().map(|review| review.url.clone()))
            .or(compare_url.clone());

        Ok(CommandReport {
            area: "ci",
            status: "ok",
            exit_status: ExitStatus::Success,
            command_path: request.command_path,
            operands: request.operands,
            output_mode: request.output_mode,
            dry_run: request.dry_run,
            summary: if created_review.is_some() {
                format!("created hosted review for `{}`.", submission.id)
            } else if pr_url.is_some() {
                format!("reported forge submission URL for `{}`.", submission.id)
            } else {
                format!(
                    "reported local submission reference for `{}`.",
                    submission.id
                )
            },
            details: Some(json!({
                "submission_id": submission.id,
                "mode": submission.mode,
                "state": submission.state,
                "branch_name": submission.branch_name,
                "packages_repo_path": submission.packages_repo_path,
                "origin_url": origin_url,
                "remote_name": remote_name,
                "remote_url": submission.remote_url,
                "target_branch": target_branch,
                "pushed_ref": submission.pushed_ref,
                "pushed_commit": submission.pushed_commit,
                "pushed_at": submission.pushed_at,
                "review_kind": submission.review_kind,
                "review_id": submission.review_id,
                "review_created_at": submission.review_created_at,
                "pr_url": pr_url,
                "compare_url": compare_url,
                "auto_open": submission_config.auto_open,
                "auto_assign": submission_config.auto_assign,
            })),
        })
    }
}

fn maybe_create_hosted_review_from_config(
    submission_config: &crate::config::ResolvedSubmissionConfig,
    submission: &CiSubmissionRecord,
    origin_url: Option<&str>,
    target_branch: &str,
) -> Result<Option<HostedReviewResult>, CoreError> {
    if submission.review_url.is_some() {
        return Ok(None);
    }

    let origin_url = match origin_url {
        Some(origin_url) => origin_url,
        None => return Ok(None),
    };

    match submission_config.auth {
        SubmissionAuthKind::None | SubmissionAuthKind::Ssh => Ok(None),
        SubmissionAuthKind::Token => {
            if submission_config.token_env.trim().is_empty() {
                return Err(CoreError::Operator(
                    "submission auth is `token` but the resolved token env is empty".to_owned(),
                ));
            }
            let token = env::var(&submission_config.token_env).map_err(|_| {
                CoreError::Operator(format!(
                    "submission auth is `token` but env var `{}` is not set",
                    submission_config.token_env
                ))
            })?;
            let title = review_title(submission);
            let description = review_body(submission);
            let request = HostedReviewRequest {
                origin_url,
                api_base_override: submission_config.api_base.as_deref(),
                branch_name: &submission.branch_name,
                target_branch,
                title: &title,
                description: &description,
                token: token.trim(),
            };
            create_hosted_review(request).map(Some)
        }
    }
}

fn review_title(submission: &CiSubmissionRecord) -> String {
    if let Some(batch_name) = &submission.batch_name {
        return format!("Elda CI batch: {batch_name}");
    }
    if submission.requested_targets.len() == 1 {
        return format!("Elda CI: {}", submission.requested_targets[0]);
    }
    format!(
        "Elda CI submission: {}",
        submission.requested_targets.join(", ")
    )
}

fn review_body(submission: &CiSubmissionRecord) -> String {
    format!(
        "Elda submission `{}`\n\nTargets: {}\nPackages: {}\nBranch: {}\n",
        submission.id,
        submission.requested_targets.join(", "),
        submission.packages.join(", "),
        submission.branch_name,
    )
}

fn create_hosted_review(request: HostedReviewRequest<'_>) -> Result<HostedReviewResult, CoreError> {
    let repo = parse_hosted_forge_repo(request.origin_url, request.api_base_override)?;
    let agent = ureq::AgentBuilder::new().build();

    match repo.kind {
        HostedForgeKind::GitHub | HostedForgeKind::Gitea => {
            let api_base = default_api_base(&repo, request.api_base_override);
            let endpoint = format!("{api_base}/repos/{}/{}/pulls", repo.owner, repo.repo);
            let body = serde_json::to_string(&json!({
                "title": request.title,
                "head": request.branch_name,
                "base": request.target_branch,
                "body": request.description,
            }))?;
            let response = agent
                .post(&endpoint)
                .set("Authorization", &format!("token {}", request.token))
                .set("Accept", "application/json")
                .set("User-Agent", "elda")
                .set("Content-Type", "application/json")
                .send_string(&body)
                .map_err(http_error)?;
            let body: serde_json::Value = serde_json::from_reader(response.into_reader())?;
            Ok(HostedReviewResult {
                url: body
                    .get("html_url")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| {
                        CoreError::Operator("forge review response missing `html_url`".to_owned())
                    })?
                    .to_owned(),
                kind: if repo.kind == HostedForgeKind::GitHub {
                    "github-pr"
                } else {
                    "gitea-pr"
                },
                review_id: body
                    .get("number")
                    .and_then(|value| value.as_u64())
                    .map(|value| value.to_string()),
            })
        }
        HostedForgeKind::GitLab => {
            let api_base = default_api_base(&repo, request.api_base_override);
            let endpoint = format!(
                "{api_base}/projects/{}/merge_requests",
                percent_encode(&repo.path)
            );
            let body = serde_json::to_string(&json!({
                "title": request.title,
                "description": request.description,
                "source_branch": request.branch_name,
                "target_branch": request.target_branch,
            }))?;
            let response = agent
                .post(&endpoint)
                .set("PRIVATE-TOKEN", request.token)
                .set("Accept", "application/json")
                .set("User-Agent", "elda")
                .set("Content-Type", "application/json")
                .send_string(&body)
                .map_err(http_error)?;
            let body: serde_json::Value = serde_json::from_reader(response.into_reader())?;
            Ok(HostedReviewResult {
                url: body
                    .get("web_url")
                    .and_then(|value| value.as_str())
                    .ok_or_else(|| {
                        CoreError::Operator("forge review response missing `web_url`".to_owned())
                    })?
                    .to_owned(),
                kind: "gitlab-mr",
                review_id: body
                    .get("iid")
                    .and_then(|value| value.as_u64())
                    .map(|value| value.to_string()),
            })
        }
    }
}

fn parse_hosted_forge_repo(
    origin_url: &str,
    api_base_override: Option<&str>,
) -> Result<HostedForgeRepo, CoreError> {
    let trimmed = origin_url
        .trim()
        .strip_suffix(".git")
        .unwrap_or(origin_url.trim());
    let (host, path) = if let Some(rest) = trimmed.strip_prefix("https://") {
        split_host_path(rest)?
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        split_host_path(rest)?
    } else if let Some(rest) = trimmed.strip_prefix("git@") {
        let (host, path) = rest.split_once(':').ok_or_else(|| {
            CoreError::Operator(format!("unsupported forge origin URL `{origin_url}`"))
        })?;
        (host.to_owned(), path.to_owned())
    } else {
        return Err(CoreError::Operator(format!(
            "unsupported forge origin URL `{origin_url}`"
        )));
    };

    let segments = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return Err(CoreError::Operator(format!(
            "forge origin `{origin_url}` does not include owner/repo information"
        )));
    }

    let kind = infer_forge_kind(&host, api_base_override)
        .ok_or_else(|| CoreError::Operator(format!("unsupported forge host `{host}`")))?;

    Ok(HostedForgeRepo {
        kind,
        host,
        path: path.clone(),
        owner: segments[segments.len() - 2].to_owned(),
        repo: segments[segments.len() - 1].to_owned(),
    })
}

fn split_host_path(value: &str) -> Result<(String, String), CoreError> {
    let (host, path) = value.split_once('/').ok_or_else(|| {
        CoreError::Operator(format!("unsupported forge origin URL `https://{value}`"))
    })?;
    Ok((host.to_owned(), path.to_owned()))
}

fn infer_forge_kind(host: &str, api_base_override: Option<&str>) -> Option<HostedForgeKind> {
    if host == "github.com" || host.contains("github") {
        return Some(HostedForgeKind::GitHub);
    }
    if host.contains("gitlab") {
        return Some(HostedForgeKind::GitLab);
    }
    if host.contains("gitea") {
        return Some(HostedForgeKind::Gitea);
    }

    let api_base = api_base_override?;
    if api_base.contains("/api/v4") {
        return Some(HostedForgeKind::GitLab);
    }
    if api_base.contains("/api/v1") {
        return Some(HostedForgeKind::Gitea);
    }

    None
}

fn default_api_base(repo: &HostedForgeRepo, api_base_override: Option<&str>) -> String {
    if let Some(api_base_override) = api_base_override {
        return api_base_override.trim_end_matches('/').to_owned();
    }

    match repo.kind {
        HostedForgeKind::GitHub => "https://api.github.com".to_owned(),
        HostedForgeKind::GitLab => format!("https://{}/api/v4", repo.host),
        HostedForgeKind::Gitea => format!("https://{}/api/v1", repo.host),
    }
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn http_error(error: ureq::Error) -> CoreError {
    match error {
        ureq::Error::Status(status, response) => {
            let body = response.into_string().unwrap_or_default();
            CoreError::Operator(format!(
                "forge review request failed with HTTP {status}: {}",
                body.trim()
            ))
        }
        ureq::Error::Transport(error) => {
            CoreError::Operator(format!("forge review transport failure: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    use super::{HostedReviewRequest, create_hosted_review, parse_hosted_forge_repo};

    #[derive(Debug)]
    struct CapturedRequest {
        method: String,
        path: String,
        headers: Vec<String>,
        body: String,
    }

    #[test]
    fn create_hosted_review_posts_github_pull_request_payload() {
        let (api_base, receiver) = start_mock_server(
            r#"{"html_url":"https://github.com/yoka-ci/pkgs/pull/42","number":42}"#,
        );
        let review = create_hosted_review(HostedReviewRequest {
            origin_url: "https://github.com/yoka-ci/pkgs.git",
            api_base_override: Some(&api_base),
            branch_name: "elda/review-tool",
            target_branch: "stable",
            title: "Elda CI: review-tool",
            description: "body",
            token: "secret-token",
        })
        .expect("github review should succeed");

        let captured = receiver.recv().expect("request should be captured");
        assert_eq!(captured.method, "POST");
        assert_eq!(captured.path, "/repos/yoka-ci/pkgs/pulls");
        assert!(
            captured
                .headers
                .iter()
                .any(|header| header == "authorization: token secret-token")
        );
        assert!(captured.body.contains("\"head\":\"elda/review-tool\""));
        assert!(captured.body.contains("\"base\":\"stable\""));
        assert_eq!(review.url, "https://github.com/yoka-ci/pkgs/pull/42");
        assert_eq!(review.review_id.as_deref(), Some("42"));
    }

    #[test]
    fn create_hosted_review_posts_gitlab_merge_request_payload() {
        let (api_base, receiver) = start_mock_server(
            r#"{"web_url":"https://gitlab.example.com/group/sub/pkgs/-/merge_requests/8","iid":8}"#,
        );
        let review = create_hosted_review(HostedReviewRequest {
            origin_url: "https://gitlab.example.com/group/sub/pkgs.git",
            api_base_override: Some(&format!("{api_base}/api/v4")),
            branch_name: "elda/review-tool",
            target_branch: "stable",
            title: "Elda CI: review-tool",
            description: "body",
            token: "secret-token",
        })
        .expect("gitlab review should succeed");

        let captured = receiver.recv().expect("request should be captured");
        assert_eq!(captured.method, "POST");
        assert_eq!(
            captured.path,
            "/api/v4/projects/group%2Fsub%2Fpkgs/merge_requests"
        );
        assert!(
            captured
                .headers
                .iter()
                .any(|header| header == "private-token: secret-token")
        );
        assert!(
            captured
                .body
                .contains("\"source_branch\":\"elda/review-tool\"")
        );
        assert_eq!(
            review.url,
            "https://gitlab.example.com/group/sub/pkgs/-/merge_requests/8"
        );
        assert_eq!(review.review_id.as_deref(), Some("8"));
    }

    #[test]
    fn parse_hosted_forge_repo_accepts_gitea_api_override() {
        let repo = parse_hosted_forge_repo(
            "https://codeberg.org/yoka-ci/pkgs.git",
            Some("https://codeberg.org/api/v1"),
        )
        .expect("gitea override should be accepted");

        assert_eq!(repo.owner, "yoka-ci");
        assert_eq!(repo.repo, "pkgs");
    }

    fn start_mock_server(response_body: &'static str) -> (String, mpsc::Receiver<CapturedRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
        let address = listener.local_addr().expect("listener addr should exist");
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("connection should arrive");
            let mut buffer = Vec::new();
            let mut header_end = None;
            loop {
                let mut chunk = [0u8; 1024];
                let read = stream.read(&mut chunk).expect("request should read");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
                if let Some(position) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
                    header_end = Some(position + 4);
                    break;
                }
            }

            let header_end = header_end.expect("request headers should end");
            let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    (name.eq_ignore_ascii_case("content-length"))
                        .then(|| value.trim().parse::<usize>().ok())?
                })
                .unwrap_or(0);
            while buffer.len() < header_end + content_length {
                let mut chunk = [0u8; 1024];
                let read = stream.read(&mut chunk).expect("request body should read");
                if read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..read]);
            }
            let body = String::from_utf8_lossy(&buffer[header_end..header_end + content_length])
                .to_string();
            let mut header_lines = headers.lines();
            let request_line = header_lines.next().expect("request line should exist");
            let mut request_parts = request_line.split_whitespace();
            let method = request_parts.next().unwrap_or_default().to_owned();
            let path = request_parts.next().unwrap_or_default().to_owned();
            sender
                .send(CapturedRequest {
                    method,
                    path,
                    headers: header_lines
                        .filter_map(|line| {
                            line.split_once(':').map(|(name, value)| {
                                format!("{}: {}", name.trim().to_ascii_lowercase(), value.trim())
                            })
                        })
                        .collect(),
                    body,
                })
                .expect("request should send");

            let response = format!(
                "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body,
            );
            stream
                .write_all(response.as_bytes())
                .expect("response should write");
        });

        (format!("http://{address}"), receiver)
    }
}
