-- 10-meta-anchor / yoka-desktop-meta
--
-- "Meta" anchor package. It carries no payload of its own; its job is to
-- pull in a curated dependency closure. `kind = "meta"` tells Elda to skip
-- archive fetch / build / staging, but the dep edges are first-class and the
-- normal solver rules apply.
pkg = {
  name = "yoka-desktop-meta",
  description = "Yoka desktop bundle (compositor, audio stack, browser, terminal).",
  licenses = { "Apache-2.0" },
  upstream = "https://yoka.invalid/desktop",
  epoch = 0,
  version = "0.4.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "meta",

  source = {
    -- Meta packages still need a `source` block for provenance, but it can
    -- point at the curating repo rather than at any payload.
    kind = "git",
    url = "https://example.invalid/yoka-desktop-meta.git",
    tag = "v0.4.0",
  },

  depends = {
    "hyprland",
    "wayland",
    "pipewire",
    "wireplumber",
    "foot",
    "thunar",
    "firefox",
  },
  makedepends = {},
  checkdepends = {},
  recommends = {
    "swaync",
    "waybar",
    "rofi-wayland",
  },
  suggests = {},
  supplements = {},
  enhances = {},
  provides = { "desktop-environment" },
  conflicts = {},
  replaces = {},
  conffiles = {},
}
