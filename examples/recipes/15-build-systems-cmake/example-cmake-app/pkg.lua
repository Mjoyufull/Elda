-- 15-build-systems-cmake / example-cmake-app
--
-- Declarative cmake build. `features` are forwarded as `-D<key>=<value>`.
pkg = {
  name = "example-cmake-app",
  description = "Example app built with the declarative cmake backend.",
  licenses = { "Apache-2.0" },
  upstream = "https://example.invalid/example-cmake-app",
  epoch = 0,
  version = "0.5.0",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://example.invalid/example-cmake-app.git",
    tag = "v0.5.0",
  },

  build = {
    system = "cmake",
    bins = { "example-cmake-app" },
    features = {
      "CMAKE_BUILD_TYPE=Release",
      "BUILD_SHARED_LIBS=ON",
      "WITH_TESTS=ON",
    },
    tests = true,
  },

  depends = {
    "openssl>=3",
  },
  makedepends = {
    "cmake>=3.20",
    "ninja",
    "pkgconf",
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
