-- 09-flag-suite-extended / flag-demo
--
-- Annotated walkthrough of every extended flag-system surface (SPEC §7):
--
--   * flags_default     - default values for every declared flag
--   * flags_allowed     - allowlist of acceptable flag names; anything not
--                         declared in default/allowed/implies/conflicts is a
--                         metadata error if any layer (global/profile/package/
--                         CLI) tries to set it
--   * flags_implies     - one-way "if A then also B" wiring
--   * flags_conflicts   - one-way "if A then NOT B" wiring
--   * flags_descriptions       - per-flag help text surfaced in `elda fl check`
--   * flags_required_one_of    - exactly one of the listed flags must be true
--   * flags_required_at_most_one - zero or one of the listed flags may be true
--   * flags_required_any_of    - at least one of the listed flags must be true
--   * conditional dependency entries with `when = "+flag,-flag"`
--
-- Drives:
--   elda i flag-demo --use=+wayland,+gpu_amd
--   elda fl check flag-demo --use=+vulkan
--   elda fl diff flag-demo
--   elda u --rebuild-variant-drift
pkg = {
  name = "flag-demo",
  description = "Reference recipe exercising every extended flag-system surface.",
  licenses = { "Apache-2.0" },
  upstream = "https://example.invalid/flag-demo",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://example.invalid/flag-demo.git",
    branch = "main",
  },

  depends = {
    "shared-runtime",
    { name = "wayland-runtime", when = "+wayland" },
    { name = "x11-runtime",     when = "+x11" },
    { name = "pipewire-libs",   when = "+pipewire,-jack" },
    { any = { "media-codecs", "media-codecs-free" }, when = "+media" },
  },
  makedepends = {
    "rust-toolchain",
    { name = "vulkan-headers", when = "+vulkan" },
  },
  checkdepends = {},
  recommends = {
    { name = "flag-demo-extras", when = "+gui" },
  },
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},

  conffiles = {},
  -- `sysusers`, `tmpfiles`, `alternatives`, `hooks`, and `provider_assets`
  -- are intentionally omitted: an empty `{}` is treated as a wrong-shape
  -- value by the validator. Leave them off when you have nothing to declare.

  flags_default = {
    wayland   = true,
    x11       = false,
    pipewire  = true,
    jack      = false,
    media     = true,
    vulkan    = false,
    gui       = true,
    gpu_intel = false,
    gpu_amd   = false,
    gpu_nvidia = false,
  },
  flags_allowed = {
    wayland   = true,
    x11       = true,
    pipewire  = true,
    jack      = true,
    media     = true,
    vulkan    = true,
    gui       = true,
    gpu_intel = true,
    gpu_amd   = true,
    gpu_nvidia = true,
    headless  = true,
  },
  flags_descriptions = {
    wayland    = "Build the Wayland compositor backend.",
    x11        = "Build the X11/XWayland fallback backend.",
    pipewire   = "Use PipeWire for audio capture and playback.",
    jack       = "Use the JACK low-latency audio bridge instead of PipeWire.",
    media      = "Pull in optional codec packs (free or non-free).",
    vulkan     = "Enable the Vulkan rendering pipeline (requires headers at build).",
    gui        = "Ship the optional GUI front-end and recommended tooling.",
    gpu_intel  = "Enable Intel-specific GPU acceleration paths.",
    gpu_amd    = "Enable AMDGPU/Mesa acceleration paths.",
    gpu_nvidia = "Enable proprietary NVIDIA acceleration paths.",
    headless   = "Disable every windowing/audio integration (server profile).",
  },
  flags_implies = {
    gpu_intel = { "vulkan" },
    gpu_amd   = { "vulkan" },
  },
  flags_conflicts = {
    wayland  = { "headless" },
    x11      = { "headless" },
    pipewire = { "jack", "headless" },
    jack     = { "pipewire" },
  },
  flags_required_one_of = {
    display = { "wayland", "x11", "headless" },
  },
  flags_required_at_most_one = {
    audio = { "pipewire", "jack" },
  },
  flags_required_any_of = {
    gpu = { "gpu_intel", "gpu_amd", "gpu_nvidia" },
  },
}
