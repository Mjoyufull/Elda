-- 06-vendor-url-archive / yt-dlp
--
-- Direct vendor / upstream archive fetch. `url_archive` covers any tarball
-- that is not hosted as a GitHub release asset; it accepts the same
-- extraction-selection fields (`strip_components`, `subdir`, `binary`,
-- `rename`) as `github_release`.
pkg = {
  name = "yt-dlp",
  description = "Feature-rich command-line audio/video downloader.",
  licenses = { "Unlicense" },
  upstream = "https://github.com/yt-dlp/yt-dlp",
  epoch = 0,
  version = "2026.04.01",
  rel = 1,
  arch = { "amd64" },
  kind = "normal",

  source = {
    kind = "url_archive",
    url = "https://example.invalid/vendor/yt-dlp/2026.04.01/yt-dlp-2026.04.01.tar.gz",
    sha256 = "4444444444444444444444444444444444444444444444444444444444444444",
    strip_components = 1,
    subdir = "yt-dlp",
    binary = "yt-dlp",
    rename = "yt-dlp",
  },

  depends = {
    "python>=3.9",
    "ffmpeg",
  },
  makedepends = {},
  checkdepends = {},
  recommends = {
    "atomicparsley",
  },
  suggests = {
    "phantomjs",
  },
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {
    "youtube-dl",
  },
  replaces = {
    "youtube-dl-legacy",
  },
  conffiles = {},
}
