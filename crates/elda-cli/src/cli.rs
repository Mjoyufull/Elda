#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};
use elda_core::{CommandRequest, OutputMode};

#[derive(Debug, Parser)]
#[command(
    name = "elda",
    bin_name = "elda",
    version,
    about = "Unix-first, Linux-first system package manager"
)]
pub struct Cli {
    #[command(flatten)]
    global: GlobalArgs,
    #[command(subcommand)]
    command: Option<Command>,
}

impl Cli {
    #[must_use]
    pub fn command_request(&self) -> Option<CommandRequest> {
        self.command.as_ref().map(|command| {
            let (command_path, operands) = command.request_parts();
            CommandRequest::new(
                command_path,
                operands,
                self.global.output_mode(),
                self.global.dry_run,
            )
        })
    }
}

#[derive(Debug, Args)]
struct GlobalArgs {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    dry_run: bool,
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
    I(TargetsArgs),
    Rm(RemoveArgs),
    U(OptionalTargetsArgs),
    Sync,
    Ls,
    Search(SearchArgs),
    Info(PackageArg),
    Files(FilesArgs),
    Verify(TargetsArgs),
    Reverify(PackageArg),
    Why(PackageArg),
    Rdeps(RdepsArgs),
    Pin(PackageArg),
    Unpin(PackageArg),
    Hold(HoldArgs),
    Unhold(PackageArg),
    Adopt(AdoptArgs),
    Downgrade(DowngradeArgs),
    Diff(DiffArgs),
    Check,
    Recover,
    Rollback(RollbackArgs),
    #[command(name = "fix-triggers")]
    FixTriggers,
    Autoremove,
    Rmt {
        #[command(subcommand)]
        command: RemoteCommand,
    },
    Rc {
        #[command(subcommand)]
        command: RecipeCommand,
    },
    Ci {
        #[command(subcommand)]
        command: CiCommand,
    },
    Vendor {
        #[command(subcommand)]
        command: VendorCommand,
    },
    Forge {
        #[command(subcommand)]
        command: ForgeCommand,
    },
    Pf {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Fl {
        #[command(subcommand)]
        command: FlagCommand,
    },
    Mg {
        #[command(subcommand)]
        command: MigrationCommand,
    },
    State {
        #[command(subcommand)]
        command: StateCommand,
    },
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    Ext {
        #[command(subcommand)]
        command: ExtensionCommand,
    },
    Qa {
        #[command(subcommand)]
        command: QaCommand,
    },
}

impl Command {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::I(args) => (vec![command_name(self)], args.targets.clone()),
            Self::Rm(args) => {
                let mut operands = args.packages.clone();
                push_flag(&mut operands, "--cascade", args.cascade);
                push_flag(&mut operands, "--purge-conffiles", args.purge_conffiles);
                (vec![command_name(self)], operands)
            }
            Self::U(args) => (vec![command_name(self)], args.targets.clone()),
            Self::Sync => (vec![command_name(self)], Vec::new()),
            Self::Ls => (vec![command_name(self)], Vec::new()),
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
            Self::Files(args) => {
                if let Some(subcommand) = &args.command {
                    match subcommand {
                        FilesSubcommand::Owner(owner) => (
                            vec![command_name(self), "owner".to_owned()],
                            vec![path_to_string(&owner.path)],
                        ),
                    }
                } else {
                    let operands = args.package.clone().into_iter().collect();
                    (vec![command_name(self)], operands)
                }
            }
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
            Self::Check | Self::Recover | Self::Autoremove => {
                (vec![command_name(self)], Vec::new())
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

fn command_name(command: &Command) -> String {
    match command {
        Command::I(_) => "i",
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

#[derive(Debug, Args)]
struct TargetsArgs {
    #[arg(value_name = "TARGET")]
    targets: Vec<String>,
}

#[derive(Debug, Args)]
struct OptionalTargetsArgs {
    #[arg(value_name = "PKG")]
    targets: Vec<String>,
}

#[derive(Debug, Args)]
struct RemoveArgs {
    #[arg(value_name = "PKG")]
    packages: Vec<String>,
    #[arg(long)]
    cascade: bool,
    #[arg(long = "purge-conffiles")]
    purge_conffiles: bool,
}

#[derive(Debug, Args)]
struct SearchArgs {
    query: String,
    #[arg(long)]
    regex: bool,
}

#[derive(Debug, Args)]
struct PackageArg {
    package: String,
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
struct FilesArgs {
    #[arg(value_name = "PKG")]
    package: Option<String>,
    #[command(subcommand)]
    command: Option<FilesSubcommand>,
}

#[derive(Debug, Subcommand)]
enum FilesSubcommand {
    Owner(FilesOwnerArgs),
}

#[derive(Debug, Args)]
struct FilesOwnerArgs {
    path: PathBuf,
}

#[derive(Debug, Args)]
struct RdepsArgs {
    package: String,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    weak: bool,
}

#[derive(Debug, Args)]
struct HoldArgs {
    package: String,
    #[arg(long)]
    source: Option<String>,
}

#[derive(Debug, Args)]
struct AdoptArgs {
    #[arg(long)]
    from: String,
    package: String,
}

#[derive(Debug, Args)]
struct DowngradeArgs {
    package: String,
    version: Option<String>,
}

#[derive(Debug, Args)]
struct DiffArgs {
    package: String,
    #[arg(long)]
    candidate: bool,
}

#[derive(Debug, Args)]
struct RollbackArgs {
    state_id: Option<String>,
}

#[derive(Debug, Subcommand)]
enum RemoteCommand {
    Add(PathArg),
}

impl RemoteCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => (
                vec!["rmt".to_owned(), "add".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum RecipeCommand {
    Add(PathArg),
    Edit(PackageArg),
    Check(OptionalPackageArg),
}

impl RecipeCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => (
                vec!["rc".to_owned(), "add".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Edit(args) => (
                vec!["rc".to_owned(), "edit".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Check(args) => (
                vec!["rc".to_owned(), "check".to_owned()],
                args.package.clone().into_iter().collect(),
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum CiCommand {
    Sub(TargetsArgs),
    Run(TargetsArgs),
    Status(OptionalValueArg),
    Pr(OptionalValueArg),
    Retry(OptionalValueArg),
    Logs(OptionalValueArg),
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },
}

impl CiCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Sub(args) => (
                vec!["ci".to_owned(), "sub".to_owned()],
                args.targets.clone(),
            ),
            Self::Run(args) => (
                vec!["ci".to_owned(), "run".to_owned()],
                args.targets.clone(),
            ),
            Self::Status(args) => (
                vec!["ci".to_owned(), "status".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Pr(args) => (
                vec!["ci".to_owned(), "pr".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Retry(args) => (
                vec!["ci".to_owned(), "retry".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Logs(args) => (
                vec!["ci".to_owned(), "logs".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Batch { command } => command.request_parts(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum BatchCommand {
    New(OptionalValueArg),
    Add(TargetsArgs),
    Push(OptionalValueArg),
}

impl BatchCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::New(args) => (
                vec!["ci".to_owned(), "batch".to_owned(), "new".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Add(args) => (
                vec!["ci".to_owned(), "batch".to_owned(), "add".to_owned()],
                args.targets.clone(),
            ),
            Self::Push(args) => (
                vec!["ci".to_owned(), "batch".to_owned(), "push".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum VendorCommand {
    Add(PathArg),
    Import(PathArg),
    Export(PathArg),
}

impl VendorCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => (
                vec!["vendor".to_owned(), "add".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Import(args) => (
                vec!["vendor".to_owned(), "import".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Export(args) => (
                vec!["vendor".to_owned(), "export".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum ForgeCommand {
    Search(ValueArg),
    Browse(ValueArg),
}

impl ForgeCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Search(args) => (
                vec!["forge".to_owned(), "search".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Browse(args) => (
                vec!["forge".to_owned(), "browse".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    Apply(TargetsArgs),
    Show,
    #[command(name = "set-init")]
    SetInit(ValueArg),
}

impl ProfileCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Apply(args) => (
                vec!["pf".to_owned(), "apply".to_owned()],
                args.targets.clone(),
            ),
            Self::Show => (vec!["pf".to_owned(), "show".to_owned()], Vec::new()),
            Self::SetInit(args) => (
                vec!["pf".to_owned(), "set-init".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum FlagCommand {
    Check(OptionalPackageArg),
    Diff(OptionalPackageArg),
}

impl FlagCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Check(args) => (
                vec!["fl".to_owned(), "check".to_owned()],
                args.package.clone().into_iter().collect(),
            ),
            Self::Diff(args) => (
                vec!["fl".to_owned(), "diff".to_owned()],
                args.package.clone().into_iter().collect(),
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum MigrationCommand {
    From(ValueArg),
    Lock(ValueArg),
    Unlock(ValueArg),
}

impl MigrationCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::From(args) => (
                vec!["mg".to_owned(), "from".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Lock(args) => (
                vec!["mg".to_owned(), "lock".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Unlock(args) => (
                vec!["mg".to_owned(), "unlock".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum StateCommand {
    Show,
    Export(OptionalPathArg),
    Import(PathBufArg),
}

impl StateCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Show => (vec!["state".to_owned(), "show".to_owned()], Vec::new()),
            Self::Export(args) => (
                vec!["state".to_owned(), "export".to_owned()],
                args.path
                    .as_ref()
                    .map(|path| vec![path_to_string(path)])
                    .unwrap_or_default(),
            ),
            Self::Import(args) => (
                vec!["state".to_owned(), "import".to_owned()],
                vec![path_to_string(&args.path)],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
enum CacheCommand {
    Add(PathArg),
    Ls,
}

impl CacheCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => (
                vec!["cache".to_owned(), "add".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Ls => (vec!["cache".to_owned(), "ls".to_owned()], Vec::new()),
        }
    }
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    Run,
    Status,
    Refresh,
}

impl DaemonCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Run => (vec!["daemon".to_owned(), "run".to_owned()], Vec::new()),
            Self::Status => (vec!["daemon".to_owned(), "status".to_owned()], Vec::new()),
            Self::Refresh => (vec!["daemon".to_owned(), "refresh".to_owned()], Vec::new()),
        }
    }
}

#[derive(Debug, Subcommand)]
enum ExtensionCommand {
    Ls,
}

impl ExtensionCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Ls => (vec!["ext".to_owned(), "ls".to_owned()], Vec::new()),
        }
    }
}

#[derive(Debug, Subcommand)]
enum QaCommand {
    Lint(OptionalValueArg),
    Build(OptionalValueArg),
    Smoke(OptionalValueArg),
    Stack(OptionalValueArg),
    Repro(OptionalValueArg),
    Diff(OptionalValueArg),
}

impl QaCommand {
    fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Lint(args) => (
                vec!["qa".to_owned(), "lint".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Build(args) => (
                vec!["qa".to_owned(), "build".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Smoke(args) => (
                vec!["qa".to_owned(), "smoke".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Stack(args) => (
                vec!["qa".to_owned(), "stack".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Repro(args) => (
                vec!["qa".to_owned(), "repro".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Diff(args) => (
                vec!["qa".to_owned(), "diff".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
        }
    }
}

#[derive(Debug, Args)]
struct ValueArg {
    value: String,
}

#[derive(Debug, Args)]
struct OptionalValueArg {
    value: Option<String>,
}

#[derive(Debug, Args)]
struct PathArg {
    value: String,
}

#[derive(Debug, Args)]
struct OptionalPackageArg {
    package: Option<String>,
}

#[derive(Debug, Args)]
struct OptionalPathArg {
    path: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct PathBufArg {
    path: PathBuf,
}

fn push_flag(operands: &mut Vec<String>, flag: &str, enabled: bool) {
    if enabled {
        operands.push(flag.to_owned());
    }
}

fn push_optional(operands: &mut Vec<String>, flag: &str, value: Option<&str>) {
    if let Some(value) = value {
        operands.push(flag.to_owned());
        operands.push(value.to_owned());
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn root_help_contains_canonical_namespaces() {
        let command = Cli::command();
        let names = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect::<Vec<_>>();

        for expected in [
            "i",
            "rm",
            "u",
            "sync",
            "ls",
            "search",
            "info",
            "files",
            "verify",
            "reverify",
            "why",
            "rdeps",
            "pin",
            "unpin",
            "hold",
            "unhold",
            "adopt",
            "downgrade",
            "diff",
            "check",
            "recover",
            "rollback",
            "fix-triggers",
            "autoremove",
            "rmt",
            "rc",
            "ci",
            "vendor",
            "forge",
            "pf",
            "fl",
            "mg",
            "state",
            "cache",
            "daemon",
            "ext",
            "qa",
        ] {
            assert!(names.contains(&expected));
        }
    }

    #[test]
    fn ci_batch_namespace_contains_nested_commands() {
        let command = Cli::command();
        let ci = command
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == "ci")
            .expect("missing ci namespace");
        let batch = ci
            .get_subcommands()
            .find(|subcommand| subcommand.get_name() == "batch")
            .expect("missing ci batch namespace");
        let names = batch
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect::<Vec<_>>();

        assert!(names.contains(&"new"));
        assert!(names.contains(&"add"));
        assert!(names.contains(&"push"));
    }
}
