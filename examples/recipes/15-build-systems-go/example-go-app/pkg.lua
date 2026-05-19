-- 15-build-systems-go / example-go-app
--
-- Declarative go build. The `bins` array selects the cmd packages to build;
-- `features` are forwarded as `-tags <value>`.
pkg = {
  name = "example-go-app",
  description = "Example app built with the declarative go backend.",
  licenses = { "BSD-3-Clause" },
  upstream = "https://example.invalid/example-go-app",
  epoch = 0,
  version = "2.0.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://example.invalid/example-go-app.git",
    tag = "v2.0.0",
  },

  build = {
    system = "go",
    bins = {
      "./cmd/example-go-app",
      "./cmd/example-go-app-helper",
    },
    features = {
      "netgo",
      "osusergo",
    },
    tests = true,
  },

  depends = {},
  makedepends = {
    "go>=1.22",
  },
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
