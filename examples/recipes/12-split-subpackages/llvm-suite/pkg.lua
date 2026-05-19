-- 12-split-subpackages / llvm-suite
--
-- Split-output recipe. One source build produces the parent package plus
-- N subpackages, each with its own dependency edges, conffiles, and flag
-- surface. The parent owns shared assets; the subpackages own their own
-- file lists, dependencies, and provides/conflicts edges.
--
-- Subpackages can be installed independently (`elda i llvm-tools`) or pulled
-- via a meta dependency (see 10-meta-anchor for that pattern).
pkg = {
  name = "llvm-suite",
  description = "Bundled LLVM/Clang stack with shared and split outputs.",
  licenses = { "Apache-2.0 WITH LLVM-exception" },
  upstream = "https://llvm.org",
  epoch = 0,
  version = "18.1.6",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "url_archive",
    url = "https://example.invalid/llvm/18.1.6/llvm-project-18.1.6.src.tar.xz",
    sha256 = "7777777777777777777777777777777777777777777777777777777777777777",
    strip_components = 1,
  },

  build = {
    system = "cmake",
    bins = {},
    features = {
      "LLVM_ENABLE_PROJECTS=clang;lld",
      "LLVM_ENABLE_RUNTIMES=compiler-rt;libcxx;libcxxabi",
      "CMAKE_BUILD_TYPE=Release",
    },
    tests = false,
  },

  depends = {
    "zlib",
    "ncurses",
  },
  makedepends = {
    "cmake>=3.20",
    "ninja",
    "python>=3.9",
  },
  checkdepends = {},
  recommends = {},
  suggests = {},
  supplements = {},
  enhances = {},
  provides = { "llvm-runtime" },
  conflicts = {},
  replaces = {},
  conffiles = {},

  subpackages = {
    {
      name = "llvm-libs",
      description = "Runtime LLVM shared libraries (libLLVM, libclang).",
      depends = { "zlib" },
      provides = { "libllvm" },
      conflicts = {},
      replaces = {},
      files = {
        "/usr/lib/libLLVM-*.so*",
        "/usr/lib/libclang-cpp.so*",
        "/usr/lib/libclang.so*",
      },
    },
    {
      name = "llvm-tools",
      description = "LLVM standalone tools (opt, llc, llvm-objdump, etc).",
      depends = { "llvm-libs" },
      provides = {},
      conflicts = {},
      replaces = {},
      files = {
        "/usr/bin/llvm-*",
        "/usr/bin/opt",
        "/usr/bin/llc",
      },
    },
    {
      name = "clang",
      description = "Clang C/C++/Objective-C compiler driver.",
      depends = { "llvm-libs", "gcc-libs" },
      provides = { "c-compiler", "cxx-compiler" },
      conflicts = {},
      replaces = {},
      files = {
        "/usr/bin/clang*",
        "/usr/bin/scan-build",
        "/usr/bin/scan-view",
        "/usr/lib/clang/**",
      },
    },
    {
      name = "lld",
      description = "LLVM linker (lld) drop-in for ld.",
      depends = { "llvm-libs" },
      provides = { "linker" },
      conflicts = {},
      replaces = {},
      files = {
        "/usr/bin/lld",
        "/usr/bin/ld.lld",
        "/usr/bin/ld64.lld",
      },
    },
  },
}
