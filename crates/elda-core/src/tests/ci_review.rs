use super::support::*;
use super::*;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn ci_pr_creates_hosted_review_via_configured_remote_and_base_branch() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let (api_base, request_log) =
        start_mock_server(r#"{"html_url":"https://github.com/yoka-ci/pkgs/pull/42","number":42}"#);
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        &format!(
            "\n[submission]\nmode = \"pr\"\nauth = \"token\"\ntoken_env = \"PATH\"\napi_base = \"{api_base}\"\nremote_name = \"upstream\"\nbase_branch = \"stable\"\n",
        ),
    );

    let source_repo = create_git_make_repo(tempdir.path(), "ci-hosted-review-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-hosted-review-tool");
    write_dual_lane_recipe(
        tempdir.path(),
        &source_repo,
        &binary,
        "ci-hosted-review-tool",
    );

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-hosted-review-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_id = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("id"))
        .and_then(|value| value.as_str())
        .expect("submission id should be reported")
        .to_owned();
    let packages_repo_path = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("packages_repo_path"))
        .and_then(|value| value.as_str())
        .expect("packages repo path should be reported")
        .to_owned();

    run_git(
        std::path::Path::new(&packages_repo_path),
        &[
            "remote",
            "add",
            "upstream",
            "https://github.com/yoka-ci/pkgs.git",
        ],
    );

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "pr".to_owned()],
            vec![submission_id],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci pr should succeed");

    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("pr_url"))
            .and_then(|value| value.as_str()),
        Some("https://github.com/yoka-ci/pkgs/pull/42")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("target_branch"))
            .and_then(|value| value.as_str()),
        Some("stable")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("review_kind"))
            .and_then(|value| value.as_str()),
        Some("github-pr")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("review_id"))
            .and_then(|value| value.as_str()),
        Some("42")
    );

    let captured = request_log.join().expect("request log thread should join");
    assert_eq!(captured.path, "/repos/yoka-ci/pkgs/pulls");
    assert!(
        captured
            .body
            .contains("\"head\":\"elda/ci-hosted-review-tool\"")
    );
    assert!(captured.body.contains("\"base\":\"stable\""));
}

#[test]
fn ci_pr_resolves_review_auth_from_submission_remote_override() {
    let tempdir = TempDir::new().expect("tempdir should be created");
    let (upstream_api, request_log) =
        start_mock_server(r#"{"html_url":"https://github.com/yoka-ci/pkgs/pull/42","number":42}"#);
    write_prefix_config_with_extras(
        tempdir.path(),
        "/opt/elda",
        true,
        false,
        &format!(
            r#"
[submission]
mode = "pr"
auth = "token"
token_env = "PATH"
api_base = "http://127.0.0.1:9"
remote_name = "origin"
base_branch = "main"

[submission.remotes.upstream]
auth = "token"
token_env = "PATH"
api_base = "{upstream_api}"
base_branch = "stable"
"#,
        ),
    );

    let source_repo = create_git_make_repo(tempdir.path(), "ci-remote-override-tool");
    let binary = create_vendor_binary(tempdir.path(), "ci-remote-override-tool");
    write_dual_lane_recipe(
        tempdir.path(),
        &source_repo,
        &binary,
        "ci-remote-override-tool",
    );

    let submission = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "sub".to_owned()],
            vec!["ci-remote-override-tool".to_owned()],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci sub should succeed");

    let submission_id = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("id"))
        .and_then(|value| value.as_str())
        .expect("submission id should be reported")
        .to_owned();
    let packages_repo_path = submission
        .details
        .as_ref()
        .and_then(|details| details.get("submission"))
        .and_then(|submission| submission.get("packages_repo_path"))
        .and_then(|value| value.as_str())
        .expect("packages repo path should be reported")
        .to_owned();
    let submissions_dir = submission
        .details
        .as_ref()
        .and_then(|details| details.get("workspace"))
        .and_then(|workspace| workspace.get("submissions_dir"))
        .and_then(|value| value.as_str())
        .expect("submissions dir should be reported")
        .to_owned();

    run_git(
        std::path::Path::new(&packages_repo_path),
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/yoka-ci/pkgs.git",
        ],
    );
    run_git(
        std::path::Path::new(&packages_repo_path),
        &[
            "remote",
            "add",
            "upstream",
            "https://github.com/yoka-ci/pkgs.git",
        ],
    );

    let submission_path =
        std::path::Path::new(&submissions_dir).join(format!("{submission_id}.json"));
    let mut record: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&submission_path).expect("submission json should read"),
    )
    .expect("submission json should parse");
    record["remote_name"] = serde_json::Value::String("upstream".to_owned());
    record["target_branch"] = serde_json::Value::String(String::new());
    fs::write(
        &submission_path,
        serde_json::to_vec_pretty(&record).expect("submission json should encode"),
    )
    .expect("submission json should write");

    let report = run_from_root(
        tempdir.path(),
        CommandRequest::new(
            vec!["ci".to_owned(), "pr".to_owned()],
            vec![submission_id],
            OutputMode::Json,
            false,
        ),
    )
    .expect("ci pr should succeed");

    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("remote_name"))
            .and_then(|value| value.as_str()),
        Some("upstream")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("target_branch"))
            .and_then(|value| value.as_str()),
        Some("stable")
    );
    assert_eq!(
        report
            .details
            .as_ref()
            .and_then(|details| details.get("pr_url"))
            .and_then(|value| value.as_str()),
        Some("https://github.com/yoka-ci/pkgs/pull/42")
    );

    let captured = request_log.join().expect("request log thread should join");
    assert_eq!(captured.path, "/repos/yoka-ci/pkgs/pulls");
    assert!(captured.body.contains("\"base\":\"stable\""));
}

struct CapturedRequest {
    path: String,
    body: String,
}

fn start_mock_server(response_body: &'static str) -> (String, thread::JoinHandle<CapturedRequest>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    let address = listener.local_addr().expect("listener addr should exist");
    let handle = thread::spawn(move || {
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

        let request_line = headers.lines().next().expect("request line should exist");
        let path = request_line
            .split_whitespace()
            .nth(1)
            .expect("request path should exist")
            .to_owned();
        let body =
            String::from_utf8_lossy(&buffer[header_end..header_end + content_length]).to_string();
        let response = format!(
            "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body,
        );
        stream
            .write_all(response.as_bytes())
            .expect("response should write");

        CapturedRequest { path, body }
    });

    (format!("http://{}", address), handle)
}
