-- build.lua runs inside Elda's embedded Lua sandbox.
--
-- Stable exported environment (SPEC §5.3):
--   pkgname, pkgver, pkgrel, srcdir, builddir, pkgdir, arch, prefix
--
-- Rules:
--   * may use staging helpers, archive/process helpers, structured logging,
--     and metadata inspection
--   * must NOT write outside the build root, spawn arbitrary processes, or
--     reach the network
--   * must stage all installed paths under `pkgdir`, never directly into /usr

elda.log.info(string.format("building %s %s for %s", pkgname, pkgver, arch))

-- Configure step. `elda.process.run` is allowed; it inherits the sandbox
-- environment and refuses to spawn programs not on the build allowlist.
elda.process.run({
  cmd = { "zig", "build", "-Doptimize=ReleaseSafe", "-Dcpu=baseline" },
  cwd = srcdir,
})

-- Stage the binary plus shell completions into the package root.
elda.staging.install_binary({
  src = builddir .. "/bin/zig-toy",
  dest = pkgdir .. prefix .. "/bin/zig-toy",
  mode = 0755,
})
elda.staging.install_file({
  src = srcdir .. "/share/completions/zig-toy.bash",
  dest = pkgdir .. prefix .. "/share/bash-completion/completions/zig-toy",
})
elda.staging.install_file({
  src = srcdir .. "/share/completions/_zig-toy",
  dest = pkgdir .. prefix .. "/share/zsh/site-functions/_zig-toy",
})

-- Optional structured note. Surfaces in `elda info zig-toy` build summary.
elda.metadata.note("staged 1 binary and 2 completion files")
