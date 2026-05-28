-- Showcase recipe for the extended flag system. Demonstrates:
--   * flag descriptions surfaced by `elda fl check`
--   * cardinality groups (`one-of`, `at-most-one`, `any-of`)
--   * conditional dependencies via the `when = "+flag,-flag"` predicate
--   * implies/conflicts wiring against the cardinality groups
--   * subpackage feature parity declarations
pkg = {
  name = "flag-suite-demo",
  epoch = 0,
  version = "1.0.0",
  rel = 1,
  arch = { "amd64", "arm64" },
  kind = "normal",

  source = {
    kind = "git",
    url = "https://example.invalid/flag-suite-demo.git",
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
    { name = "flag-suite-tools", when = "+gui" },
  },
  suggests = {},
  supplements = {},
  enhances = {},
  provides = {},
  conflicts = {},
  replaces = {},

  conffiles = {},
  -- Empty `{}` is treated as a wrong-shape value by the validator. Leave
  -- optional families (`sysusers`, `tmpfiles`, `alternatives`, `hooks`,
  -- `provider_assets`) off when there is nothing to declare.

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
    -- Operators must pick exactly one display backend.
    display = { "wayland", "x11", "headless" },
  },
  flags_required_at_most_one = {
    audio = { "pipewire", "jack" },
  },
  flags_required_any_of = {
    gpu = { "gpu_intel", "gpu_amd", "gpu_nvidia" },
  },
}
