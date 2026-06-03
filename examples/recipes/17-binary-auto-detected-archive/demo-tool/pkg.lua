-- 17-binary-auto-detected-archive / demo-tool
--
-- Binary tar archive with no explicit `binary` field. Elda verifies the
-- checksum first, scans the tar entries, and proceeds only if there is exactly
-- one executable launcher candidate. If the archive has multiple executables,
-- add `binary = "path/in/archive"` to make the selection explicit.
pkg = {
  name = "demo-tool",
  description = "Example binary archive with one auto-detected launcher.",
  licenses = { "MIT" },
  upstream = "https://example.invalid/demo-tool",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "url_archive",
    url = "https://example.invalid/demo-tool/demo-tool-1.0.0-linux-amd64.tar.gz",
    sha256 = "7777777777777777777777777777777777777777777777777777777777777777",
    rename = "demo-tool",
  },

  depends = {},
  makedepends = {},
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},
  conffiles = {},
}
