use clap::{Args, Parser, Subcommand};
use elda_core::{CommandRequest, OutputMode};

use super::common::{
    AdoptArgs, DiffArgs, FilesArgs, FilesSubcommand, HoldArgs, InstallArgs, InstallTargetsArgs,
    PackageArg, RdepsArgs, RemoveArgs, RollbackArgs, SearchArgs, TargetsArgs, UpgradeArgs,
    path_to_string, push_flag, push_optional,
};
use super::repo_commands::{CiCommand, ForgeCommand, RecipeCommand, RemoteCommand, VendorCommand};
use super::styles::clap_styles;
use super::system_commands::{
    CacheCommand, DaemonCommand, ExtensionCommand, FlagCommand, MigrationCommand, ProfileCommand,
    QaCommand, StateCommand,
};

#[derive(Debug, Parser)]
#[command(
    name = "elda",
    bin_name = "elda",
    version,
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
        long = "log-level",
        global = true,
        value_name = "1|2|3",
        help = "Write the per-run session log at level 1, 2, or 3"
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
enum Command {
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
    Sync,
    #[command(about = "List installed packages in the current root")]
    Ls,
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
}

impl Command {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::I(args) => {
                let mut operands = args.targets.clone();
                for flag in &args.use_flags {
                    operands.push(format!("--use={flag}"));
                }
                push_flag(&mut operands, "--prefer-source", args.prefer_source);
                push_flag(&mut operands, "--prefer-binary", args.prefer_binary);
                (vec![command_name(self)], operands)
            }
            Self::Ig(args) | Self::Ib(args) => {
                let mut operands = args.targets.clone();
                for flag in &args.use_flags {
                    operands.push(format!("--use={flag}"));
                }
                (vec![command_name(self)], operands)
            }
            Self::Rm(args) => {
                let mut operands = args.packages.clone();
                push_flag(&mut operands, "--cascade", args.cascade);
                push_flag(&mut operands, "--purge-conffiles", args.purge_conffiles);
                (vec![command_name(self)], operands)
            }
            Self::U(args) => {
                let mut operands = args.targets.clone();
                push_flag(&mut operands, "--refresh-weak-deps", args.refresh_weak_deps);
                (vec![command_name(self)], operands)
            }
            Self::Sync | Self::Ls | Self::Check | Self::Recover | Self::Autoremove => {
                (vec![command_name(self)], Vec::new())
            }
            Self::Search(args) => {
                let mut operands = vec![args.query.clone()];
                push_flag(&mut operands, "--regex", args.regex);
                (vec![command_name(self)], operands)
            }
            Self::Info(args)
            | Self::Reverify(args)
            | Self::Why(args)
            | Self::Pin(args)
            | Self::Unpin(args)
            | Self::Unhold(args) => (vec![command_name(self)], vec![args.package.clone()]),
            Self::Files(args) => files_request_parts(args),
            Self::Verify(args) => (vec![command_name(self)], args.targets.clone()),
            Self::Rdeps(args) => {
                let mut operands = vec![args.package.clone()];
                push_flag(&mut operands, "--all", args.all);
                push_flag(&mut operands, "--weak", args.weak);
                (vec![command_name(self)], operands)
            }
            Self::Hold(args) => {
                let mut operands = vec![args.package.clone()];
                push_optional(&mut operands, "--source", args.source.as_deref());
                (vec![command_name(self)], operands)
            }
            Self::Adopt(args) => (
                vec![command_name(self)],
                vec!["--from".to_owned(), args.from.clone(), args.package.clone()],
            ),
            Self::Downgrade(args) => {
                let mut operands = vec![args.package.clone()];
                if let Some(version) = &args.version {
                    operands.push(version.clone());
                }
                (vec![command_name(self)], operands)
            }
            Self::Diff(args) => {
                let mut operands = vec![args.package.clone()];
                push_flag(&mut operands, "--candidate", args.candidate);
                (vec![command_name(self)], operands)
            }
            Self::Rollback(args) => (
                vec![command_name(self)],
                args.state_id.clone().into_iter().collect(),
            ),
            Self::FixTriggers => (vec![command_name(self)], Vec::new()),
            Self::Rmt { command } => command.request_parts(),
            Self::Rc { command } => command.request_parts(),
            Self::Ci { command } => command.request_parts(),
            Self::Vendor { command } => command.request_parts(),
            Self::Forge { command } => command.request_parts(),
            Self::Pf { command } => command.request_parts(),
            Self::Fl { command } => command.request_parts(),
            Self::Mg { command } => command.request_parts(),
            Self::State { command } => command.request_parts(),
            Self::Cache { command } => command.request_parts(),
            Self::Daemon { command } => command.request_parts(),
            Self::Ext { command } => command.request_parts(),
            Self::Qa { command } => command.request_parts(),
        }
    }
}

fn files_request_parts(args: &FilesArgs) -> (Vec<String>, Vec<String>) {
    if let Some(subcommand) = &args.command {
        match subcommand {
            FilesSubcommand::Owner(owner) => (
                vec!["files".to_owned(), "owner".to_owned()],
                vec![path_to_string(&owner.path)],
            ),
        }
    } else {
        let operands = args.package.clone().into_iter().collect();
        (vec!["files".to_owned()], operands)
    }
}

fn command_name(command: &Command) -> String {
    match command {
        Command::I(_) => "i",
        Command::Ig(_) => "ig",
        Command::Ib(_) => "ib",
        Command::Rm(_) => "rm",
        Command::U(_) => "u",
        Command::Sync => "sync",
        Command::Ls => "ls",
        Command::Search(_) => "search",
        Command::Info(_) => "info",
        Command::Files(_) => "files",
        Command::Verify(_) => "verify",
        Command::Reverify(_) => "reverify",
        Command::Why(_) => "why",
        Command::Rdeps(_) => "rdeps",
        Command::Pin(_) => "pin",
        Command::Unpin(_) => "unpin",
        Command::Hold(_) => "hold",
        Command::Unhold(_) => "unhold",
        Command::Adopt(_) => "adopt",
        Command::Downgrade(_) => "downgrade",
        Command::Diff(_) => "diff",
        Command::Check => "check",
        Command::Recover => "recover",
        Command::Rollback(_) => "rollback",
        Command::FixTriggers => "fix-triggers",
        Command::Autoremove => "autoremove",
        Command::Rmt { .. } => "rmt",
        Command::Rc { .. } => "rc",
        Command::Ci { .. } => "ci",
        Command::Vendor { .. } => "vendor",
        Command::Forge { .. } => "forge",
        Command::Pf { .. } => "pf",
        Command::Fl { .. } => "fl",
        Command::Mg { .. } => "mg",
        Command::State { .. } => "state",
        Command::Cache { .. } => "cache",
        Command::Daemon { .. } => "daemon",
        Command::Ext { .. } => "ext",
        Command::Qa { .. } => "qa",
    }
    .to_owned()
}
