# GitLab Hosting

Use GitLab when you want integrated CI runners and GitLab-hosted review (Merge Requests).

## Topology

```text
gitlab.example.com/yoka/pkgs.git          recipe definitions
gitlab.example.com/yoka/index             raw index branch or Package Registry
packages.example.com/elda                 optional static cache
```

## Remote Registration

```sh
elda rmt add yoka-main=https://gitlab.example.com/yoka/index/-/raw/main/index-v1.json.zst \
  --trust pinned \
  --trusted-key ed25519:0011223344556677889900112233445566778899aabbccddeeff0011223344 \
  --signature-url https://gitlab.example.com/yoka/index/-/raw/main/index-v1.json.zst.sig \
  --metadata-url https://gitlab.example.com/yoka/index/-/raw/main/remote-metadata-v1.toml \
  --packages-url https://gitlab.example.com/yoka/pkgs.git
```

Self-hosted GitLab: replace `gitlab.example.com` everywhere; set `api_base` to `https://gitlab.example.com/api/v4`.

## Submission Config

```toml
[submission]
mode = "pr"
auto_open = true
auto_assign = false
auth = "token"
token_env = "ELDA_GITLAB_TOKEN"
api_base = "https://gitlab.com/api/v4"
remote_name = "origin"
base_branch = "main"
```

## Payloads

The signed Elda index remains authoritative whether payloads live in:

- GitLab Releases
- GitLab Generic Package Registry
- Object storage (S3-compatible)
- A static front (nginx, Caddy)

Record the final HTTPS URLs in the index; clients do not care which GitLab feature served the upload.

## CI Notes

- Project access token or `CI_JOB_TOKEN` for publish jobs scoped to the index project.
- `.gitlab-ci.yml` can artifact `index-v1.json.zst` and deploy to Pages or Registry on `main`.

See [../binary-binhost-remote.md](../binary-binhost-remote.md).
