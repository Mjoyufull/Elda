# Cache Server Setup

A cache is any static server that can answer:

```text
GET <cache base>/<sha256>
```

## Cache Document

```toml
name = "lan-mirror"
base_url = "https://cache.example.com/elda"
priority = 20
enabled = true
```

```sh
elda cache add lan-mirror=https://cache.example.com/elda --priority 20
elda cache ls
```

## Static Layout

```text
/var/www/elda-cache/
  6f1e...payload-digest
  7a93...payload-digest
```

## Caddy

```caddyfile
cache.example.com {
    root * /var/www/elda-cache
    file_server
}
```

## Nginx

```nginx
server {
    listen 443 ssl;
    server_name cache.example.com;
    root /var/www/elda-cache;

    location / {
        try_files $uri =404;
    }
}
```

## Filling Caches Today

```sh
elda-populate cache push-local --cache lan-mirror --installed
elda-populate cache mirror-remote --remote yoka-main --channel stable --cache lan-mirror
```

**Limitation:** direct writes are `file://` or local-path first. For production, mirror the filled directory with `rsync`, `rclone`, `scp`, or object-storage sync.

See [patterns/lan-cache-mirror-only.md](./patterns/lan-cache-mirror-only.md).
