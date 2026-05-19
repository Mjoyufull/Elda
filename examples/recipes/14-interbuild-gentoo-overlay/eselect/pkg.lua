-- 14-interbuild-gentoo-overlay / eselect
--
-- Interbuild source: a maintained recipe whose source contract delegates to a
-- Gentoo overlay ebuild (SPEC §5.2 + §12.2). The author-facing
-- `kind = "gentoo_overlay"` becomes `source_kind = interbuild` once persisted.
--
-- Required:  url, package
-- Optional:  rev, binhost, use
pkg = {
  name = "eselect",
  description = "Gentoo eselect framework (built via overlay adapter).",
  licenses = { "GPL-2.0-only" },
  upstream = "https://wiki.gentoo.org/wiki/Project:Eselect",
  epoch = 0,
  version = "1.4.28",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "gentoo_overlay",
    url = "https://github.com/gentoo/gentoo",
    package = "app-admin/eselect",
    rev = "0123456789abcdef0123456789abcdef01234567",
    binhost = "https://example.invalid/binhost/amd64",
    -- USE flags are space-separated, mirroring the upstream `USE="..."` shape.
    -- Negate with a leading `-` (e.g. `-suid` to disable that USE flag).
    use = "doc -suid",
  },

  depends = {
    "bash>=5.0",
  },
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
