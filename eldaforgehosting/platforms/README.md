# Platform Guides

Each guide explains **where to host** the recipe Git repo, signed index, payloads, and forge submission tokens for that platform.

| Platform | Guide |
| --- | --- |
| GitHub | [github.md](./github.md) |
| GitLab | [gitlab.md](./gitlab.md) |
| Gitea / Forgejo | [gitea-forgejo.md](./gitea-forgejo.md) |
| SourceHut | [sourcehut.md](./sourcehut.md) |
| Generic static HTTP / S3 / CDN | [generic-static-http.md](./generic-static-http.md) |
| Bitbucket, Codeberg, others | [bitbucket-and-others.md](./bitbucket-and-others.md) |

Platform choice does not change the Elda client contract: the **signed index** remains authoritative. Payloads may live on the same host or anywhere with stable HTTPS URLs recorded in the index.
