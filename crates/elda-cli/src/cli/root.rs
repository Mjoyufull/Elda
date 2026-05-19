use clap::{Args, Parser, Subcommand};
use elda_core::{CommandRequest, OutputMode};

use super::appimage_commands::AppImageCommand;
use super::common::{
    AdoptArgs, DiffArgs, HoldArgs, InstallArgs, InstallTargetsArgs, ListArgs, PackageArg,
    RdepsArgs, RemoveArgs, RollbackArgs, SearchArgs, TargetsArgs, UpgradeArgs,
};
use super::file_commands::FilesArgs;
use super::git_commands::{GitCommand, GitTagsArgs};
use super::host_commands::HostCommand;
use super::publish_commands::PublishCommand;
use super::repo_commands::{CiCommand, ForgeCommand, RecipeCommand, RemoteCommand, VendorCommand};
use super::request_parts;
use super::styles::clap_styles;
use super::system_commands::{
    CacheCommand, ConfigCommand, DaemonCommand, ExtensionCommand, FlagCommand, MaintCommand,
    MigrationCommand, ProfileCommand, QaCommand, ReviewCommand, StateCommand, TriggerCommand,
};

#[derive(Debug, Parser)]
#[command(
    name = "elda",
    bin_name = "elda",
    disable_version_flag = true,
    about = "Replacement-grade Unix-first package manager",
    styles = clap_styles()
)]
pub(crate) struct Cli {
    #[command(flatten)]
    global: GlobalArgs,
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    #[must_use]
    pub(crate) fn command_request(&self) -> Option<CommandRequest> {
        self.command.as_ref().map(|command| {
            let (command_path, operands) = command.request_parts();
            CommandRequest::new(
                command_path,
                operands,
                self.global.output_mode(),
                self.global.dry_run,
            )
            .with_system_mode(self.global.system)
            .with_offline(self.global.offline)
            .with_log_level(self.global.log_level)
            .with_accepted_rotated_keys(self.global.accept_rotated_keys.clone())
            .with_no_stream(self.global.no_stream)
        })
    }
}

#[derive(Debug, Args)]
struct GlobalArgs {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    dry_run: bool,
    #[arg(long, global = true)]
    offline: bool,
    #[arg(
        long = "no-stream",
        global = true,
        help = "Disable live progress event streaming and print only the final report"
    )]
    no_stream: bool,
    #[arg(
        long = "log-level",
        global = true,
        value_name = "0|1|2|3",
        value_parser = clap::value_parser!(u8).range(0..=3),
        help = "Per-run session log: 0 off, 1–3 more detail (overrides [logging].level for this run)"
    )]
    log_level: Option<u8>,
    #[arg(
        long = "accept-rotated-key",
        global = true,
        value_name = "REMOTE",
        help = "Accept one signed TOFU key rotation for the named remote"
    )]
    accept_rotated_keys: Vec<String>,
    #[arg(short = 'S', long = "system", global = true)]
    system: bool,
}

impl GlobalArgs {
    fn output_mode(&self) -> OutputMode {
        if self.json {
            OutputMode::Json
        } else {
            OutputMode::Human
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum Command {
    #[command(about = "Add package metadata from a link or local source")]
    A(InstallArgs),
    #[command(about = "Add package metadata from a link or local source")]
    Add(InstallArgs),
    #[command(about = "Install package names, local recipes, or git targets")]
    I(InstallArgs),
    #[command(about = "Install through the source lane")]
    Ig(InstallTargetsArgs),
    #[command(about = "Install through the binary lane")]
    Ib(InstallTargetsArgs),
    #[command(about = "Remove installed packages")]
    Rm(RemoveArgs),
    #[command(about = "Upgrade world or the selected package closure")]
    U(UpgradeArgs),
    #[command(about = "Refresh configured remotes into the local snapshot")]
    Sync(TargetsArgs),
    #[command(alias = "list")]
    #[command(about = "List installed packages in the current root")]
    Ls(ListArgs),
    #[command(about = "Search synced package indexes")]
    Search(SearchArgs),
    #[command(about = "Inspect installed or synced package metadata")]
    Info(PackageArg),
    #[command(about = "Inspect package file ownership")]
    Files(FilesArgs),
    #[command(about = "Verify managed files against recorded manifests")]
    Verify(TargetsArgs),
    #[command(about = "Re-run verification for one installed package")]
    Reverify(PackageArg),
    #[command(about = "Explain why a package is present")]
    Why(PackageArg),
    #[command(about = "Show reverse dependency edges")]
    Rdeps(RdepsArgs),
    #[command(about = "List git-backed upstream version candidates")]
    Versions(GitTagsArgs),
    #[command(about = "Pin a package to its current version")]
    Pin(PackageArg),
    #[command(about = "Clear an exact-version pin")]
    Unpin(PackageArg),
    #[command(about = "Block upgrades for a package")]
    Hold(HoldArgs),
    #[command(about = "Clear an upgrade hold")]
    Unhold(PackageArg),
    #[command(about = "Adopt one package from another package manager")]
    Adopt(AdoptArgs),
    #[command(about = "Install an older cached or archived version")]
    Downgrade(super::common::DowngradeArgs),
    #[command(about = "Compare live files or the next candidate manifest")]
    Diff(DiffArgs),
    #[command(about = "Show aggregated root health")]
    Check,
    #[command(about = "Check bootstrap and release-readiness state")]
    Doctor,
    #[command(about = "Show Elda build and component version details")]
    Version,
    #[command(about = "Create Elda config and data directories")]
    Init,
    #[command(about = "Repair or roll back incomplete transactions")]
    Recover,
    #[command(about = "Restore a previously archived state")]
    Rollback(RollbackArgs),
    #[command(name = "fix-triggers")]
    #[command(about = "Reconcile pending trigger work")]
    FixTriggers,
    #[command(about = "Remove orphaned dependency packages")]
    Autoremove,
    #[command(about = "Remote registration and trust bootstrap")]
    Rmt {
        #[command(subcommand)]
        command: RemoteCommand,
    },
    #[command(about = "Local recipe management")]
    Rc {
        #[command(subcommand)]
        command: RecipeCommand,
    },
    #[command(about = "Submission and binary publishing workflow")]
    Ci {
        #[command(subcommand)]
        command: CiCommand,
    },
    #[command(about = "Vendor binary recipe management")]
    Vendor {
        #[command(subcommand)]
        command: VendorCommand,
    },
    #[command(about = "Forge discovery and asset browsing")]
    Forge {
        #[command(subcommand)]
        command: ForgeCommand,
    },
    #[command(about = "Maintainer recipe tree and forge ergonomics")]
    Host {
        #[command(subcommand)]
        command: HostCommand,
    },
    #[command(about = "Signed index publication for binhosts")]
    Publish {
        #[command(subcommand)]
        command: PublishCommand,
    },
    #[command(
        name = "appimage",
        about = "Inspect portable AppImage payloads (read-only SquashFS)"
    )]
    AppImage {
        #[command(subcommand)]
        command: AppImageCommand,
    },
    #[command(about = "Git source inspection")]
    Git {
        #[command(subcommand)]
        command: GitCommand,
    },
    #[command(about = "Profile and provider management")]
    Pf {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    #[command(about = "Flag and variant inspection")]
    Fl {
        #[command(subcommand)]
        command: FlagCommand,
    },
    #[command(about = "Whole-system migration and coexistence control")]
    Mg {
        #[command(subcommand)]
        command: MigrationCommand,
    },
    #[command(about = "Source-definition review memory")]
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    #[command(about = "Desired machine state export and import")]
    State {
        #[command(subcommand)]
        command: StateCommand,
    },
    #[command(about = "Cache registration and inspection")]
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    #[command(about = "Background refresh and notification surface")]
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    #[command(about = "Extension inspection")]
    Ext {
        #[command(subcommand)]
        command: ExtensionCommand,
    },
    #[command(about = "Lint, build, smoke, and reproducibility tooling")]
    Qa {
        #[command(subcommand)]
        command: QaCommand,
    },
    #[command(about = "System trigger inspection")]
    Trigger {
        #[command(subcommand)]
        command: TriggerCommand,
    },
    #[command(about = "Recovery, triggers, and profile maintenance")]
    Maint {
        #[command(subcommand)]
        command: MaintCommand,
    },
    #[command(about = "Configuration merge queue inspection")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

impl Command {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        request_parts::request_parts(self)
    }
}
