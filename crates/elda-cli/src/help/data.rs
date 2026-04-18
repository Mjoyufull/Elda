pub(super) const ACID_LIME: (u8, u8, u8) = (171, 255, 67);
pub(super) const HAZARD_ORANGE: (u8, u8, u8) = (255, 107, 0);
pub(super) const ELECTRIC_MAGENTA: (u8, u8, u8) = (255, 0, 255);
pub(super) const BURNT_RUST: (u8, u8, u8) = (135, 48, 3);
pub(super) const PURE_WHITE: (u8, u8, u8) = (255, 255, 255);
pub(super) const SIGNAL_YELLOW: (u8, u8, u8) = (255, 255, 0);

pub(super) const LOGO: &str = include_str!("../../../../assets/ASCIIlogo.txt");

pub(super) const CORE_ROWS: &[HelpRow] = &[
    HelpRow::new(
        "i",
        "<target...>",
        "install package names, recipes, or git targets",
    ),
    HelpRow::new("ig", "<target...>", "install through the source lane"),
    HelpRow::new("ib", "<pkg...>", "install through the binary lane"),
    HelpRow::new(
        "sync",
        "",
        "refresh configured remotes into the local snapshot",
    ),
    HelpRow::new("search", "<query>", "search synced package indexes"),
    HelpRow::new(
        "info",
        "<pkg>",
        "inspect installed or synced package metadata",
    ),
    HelpRow::new("ls", "", "list installed packages in the current root"),
    HelpRow::new(
        "files",
        "<pkg>",
        "list owned paths for an installed package",
    ),
    HelpRow::new(
        "files owner",
        "<path>",
        "show which installed package owns a managed path",
    ),
];

pub(super) const STATE_ROWS: &[HelpRow] = &[
    HelpRow::new("rm", "<pkg...>", "remove installed packages"),
    HelpRow::new("u", "[pkg...]", "upgrade world or the selected closure"),
    HelpRow::new(
        "diff",
        "<pkg>",
        "compare live state or the next candidate manifest",
    ),
    HelpRow::new(
        "verify",
        "[pkg...]",
        "verify managed files against recorded manifests",
    ),
    HelpRow::new(
        "reverify",
        "<pkg>",
        "rerun verification for one installed package",
    ),
    HelpRow::new("why", "<pkg>", "explain why a package is present"),
    HelpRow::new("rdeps", "<pkg>", "show reverse dependencies"),
    HelpRow::new(
        "pin",
        "<pkg>",
        "pin an installed package to its current version",
    ),
    HelpRow::new("unpin", "<pkg>", "clear an exact-version pin"),
    HelpRow::new("hold", "<pkg>", "block upgrades for a package"),
    HelpRow::new("unhold", "<pkg>", "clear an upgrade hold"),
    HelpRow::new("check", "", "show root health, journals, and safety issues"),
    HelpRow::new("recover", "", "repair or roll back incomplete transactions"),
    HelpRow::new(
        "rollback",
        "[state-id]",
        "restore a previously archived state",
    ),
    HelpRow::new(
        "fix-triggers",
        "",
        "reconcile pending trigger work in the current slice",
    ),
    HelpRow::new("autoremove", "", "remove orphaned dependency packages"),
    HelpRow::new(
        "downgrade",
        "<pkg> [version]",
        "install an older cached or archived version",
    ),
    HelpRow::new(
        "adopt",
        "--from <pm> <pkg>",
        "adopt one package from another package manager",
    ),
];

pub(super) const NAMESPACE_ROWS: &[HelpRow] = &[
    HelpRow::new("rmt add", "<name=url>", "register a remote index"),
    HelpRow::new("rc add/edit/check", "...", "manage local recipes"),
    HelpRow::new(
        "vendor add/import/export",
        "...",
        "manage vendor binary recipes",
    ),
    HelpRow::new(
        "forge search/browse",
        "...",
        "discover forge packages and assets",
    ),
    HelpRow::new(
        "pf apply/add/rm/...",
        "...",
        "manage machine profile and provider state",
    ),
    HelpRow::new(
        "state show/export/import",
        "...",
        "inspect or move desired machine state",
    ),
    HelpRow::new("cache add/ls", "...", "register and inspect caches"),
    HelpRow::new(
        "daemon run/status/refresh",
        "...",
        "operate the background refresh surface",
    ),
    HelpRow::new("ci ...", "", "submission and binary publishing workflow"),
    HelpRow::new("fl ...", "", "flag and variant inspection"),
    HelpRow::new(
        "mg ...",
        "",
        "whole-system migration and coexistence control",
    ),
    HelpRow::new("ext ls", "", "inspect installed extension backends"),
    HelpRow::new(
        "qa ...",
        "",
        "lint, build, smoke, and reproducibility tooling",
    ),
];

pub(super) const FLAG_ROWS: &[HelpRow] = &[
    HelpRow::new("--json", "", "emit machine-readable JSON output"),
    HelpRow::new(
        "--dry-run",
        "",
        "show the planned mutation without applying it",
    ),
    HelpRow::new(
        "--offline",
        "",
        "use only cached payloads and verified local snapshots",
    ),
    HelpRow::new(
        "--accept-rotated-key <remote>",
        "",
        "accept one signed TOFU key rotation for a remote",
    ),
    HelpRow::new(
        "-S, --system",
        "",
        "request live host system mode for this invocation",
    ),
    HelpRow::new("-h, --help", "", "show this help screen"),
    HelpRow::new("-V, --version", "", "show the current Elda version"),
];

pub(super) const EXAMPLES: &[ExampleRow] = &[
    ExampleRow::new(
        "elda i ripgrep",
        "install a synced package with normal lane selection",
    ),
    ExampleRow::new(
        "elda ig https://github.com/foo/bar",
        "install a git target through the source path",
    ),
    ExampleRow::new(
        "elda vendor add fsel-bin Mjoyufull/fsel@latest --binary fsel",
        "track a GitHub release binary as a local recipe",
    ),
    ExampleRow::new(
        "elda pf apply yoka-core --init dinit --foreign-arch i386",
        "apply profile anchors and persist machine-shape policy",
    ),
    ExampleRow::new(
        "elda pf add yoka-desktop-hyprland",
        "append one profile anchor onto the current machine shape",
    ),
    ExampleRow::new(
        "elda help <command>",
        "show Clap-generated help for a specific command",
    ),
];

#[derive(Debug, Clone, Copy)]
pub(super) struct HelpRow {
    pub(super) command: &'static str,
    pub(super) args: &'static str,
    pub(super) description: &'static str,
}

impl HelpRow {
    const fn new(command: &'static str, args: &'static str, description: &'static str) -> Self {
        Self {
            command,
            args,
            description,
        }
    }

    pub(super) fn label_width(&self) -> usize {
        if self.args.is_empty() {
            self.command.chars().count()
        } else {
            self.command.chars().count() + 1 + self.args.chars().count()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ExampleRow {
    pub(super) command: &'static str,
    pub(super) description: &'static str,
}

impl ExampleRow {
    const fn new(command: &'static str, description: &'static str) -> Self {
        Self {
            command,
            description,
        }
    }
}
