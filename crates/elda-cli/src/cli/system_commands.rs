use clap::{Args, Subcommand};

use super::common::{
    ArchListArgs, OptionalPackageArg, OptionalPathArg, OptionalValueArg, PathBufArg,
    ProfileApplyArgs, ValueArg, path_to_string, push_optional,
};

#[derive(Debug, Subcommand)]
pub(super) enum ProfileCommand {
    #[command(about = "Apply one or more profile anchors")]
    Apply(ProfileApplyArgs),
    #[command(about = "Append profile anchors to the active machine shape")]
    Add(ProfileApplyArgs),
    #[command(name = "rm")]
    #[command(about = "Remove profile anchors from the active machine shape")]
    Rm(ProfileApplyArgs),
    #[command(about = "Show the currently detected machine profile state")]
    Show,
    #[command(name = "set-init")]
    #[command(about = "Switch the active init-provider family")]
    SetInit(ValueArg),
    #[command(name = "clear-init")]
    #[command(about = "Clear the active init-provider family override")]
    ClearInit,
    #[command(name = "set-arch")]
    #[command(about = "Set the active native architecture")]
    SetArch(ValueArg),
    #[command(name = "add-foreign-arch")]
    #[command(about = "Enable one or more foreign architectures")]
    AddForeignArch(ArchListArgs),
    #[command(name = "remove-foreign-arch")]
    #[command(about = "Disable one or more foreign architectures")]
    RemoveForeignArch(ArchListArgs),
}

impl ProfileCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Apply(args) => profile_selection_request_parts("apply", args),
            Self::Add(args) => profile_selection_request_parts("add", args),
            Self::Rm(args) => profile_selection_request_parts("rm", args),
            Self::Show => (vec!["pf".to_owned(), "show".to_owned()], Vec::new()),
            Self::SetInit(args) => (
                vec!["pf".to_owned(), "set-init".to_owned()],
                vec![args.value.clone()],
            ),
            Self::ClearInit => (vec!["pf".to_owned(), "clear-init".to_owned()], Vec::new()),
            Self::SetArch(args) => (
                vec!["pf".to_owned(), "set-arch".to_owned()],
                vec![args.value.clone()],
            ),
            Self::AddForeignArch(args) => (
                vec!["pf".to_owned(), "add-foreign-arch".to_owned()],
                args.arches.clone(),
            ),
            Self::RemoveForeignArch(args) => (
                vec!["pf".to_owned(), "remove-foreign-arch".to_owned()],
                args.arches.clone(),
            ),
        }
    }
}

fn profile_selection_request_parts(
    command: &str,
    args: &ProfileApplyArgs,
) -> (Vec<String>, Vec<String>) {
    let mut operands = args.targets.clone();
    push_optional(&mut operands, "--init", args.init.as_deref());
    push_optional(&mut operands, "--native-arch", args.native_arch.as_deref());
    for arch in &args.foreign_arches {
        operands.push("--foreign-arch".to_owned());
        operands.push(arch.clone());
    }

    (vec!["pf".to_owned(), command.to_owned()], operands)
}

#[derive(Debug, Subcommand)]
pub(super) enum FlagCommand {
    #[command(about = "Inspect effective flag state")]
    Check(OptionalPackageArg),
    #[command(about = "Diff effective flag state")]
    Diff(OptionalPackageArg),
}

impl FlagCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
pub(super) enum MigrationCommand {
    #[command(about = "Import installed state from another package manager")]
    From(ValueArg),
    #[command(about = "Lock a foreign package manager out of mutations")]
    Lock(ValueArg),
    #[command(about = "Release a previously applied migration lock")]
    Unlock(ValueArg),
}

impl MigrationCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
pub(super) enum StateCommand {
    #[command(about = "Show the current desired machine state")]
    Show,
    #[command(about = "Export desired machine state")]
    Export(OptionalPathArg),
    #[command(about = "Import desired machine state")]
    Import(PathBufArg),
}

impl StateCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
pub(super) enum CacheCommand {
    #[command(about = "Register a cache document")]
    Add(CacheAddArgs),
    #[command(alias = "list")]
    #[command(about = "List registered caches")]
    Ls,
}

impl CacheCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => {
                let mut operands = vec![args.value.clone()];
                if let Some(priority) = args.priority {
                    operands.push("--priority".to_owned());
                    operands.push(priority.to_string());
                }
                (vec!["cache".to_owned(), "add".to_owned()], operands)
            }
            Self::Ls => (vec!["cache".to_owned(), "ls".to_owned()], Vec::new()),
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct CacheAddArgs {
    pub(super) value: String,
    #[arg(long)]
    pub(super) priority: Option<u32>,
}

#[derive(Debug, Subcommand)]
pub(super) enum DaemonCommand {
    #[command(about = "Run the background refresh surface")]
    Run,
    #[command(about = "Show daemon configuration and runtime state")]
    Status,
    #[command(about = "Trigger an immediate refresh pass")]
    Refresh,
}

impl DaemonCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Run => (vec!["daemon".to_owned(), "run".to_owned()], Vec::new()),
            Self::Status => (vec!["daemon".to_owned(), "status".to_owned()], Vec::new()),
            Self::Refresh => (vec!["daemon".to_owned(), "refresh".to_owned()], Vec::new()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum ExtensionCommand {
    #[command(alias = "list")]
    #[command(about = "List configured extensions")]
    Ls,
}

impl ExtensionCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Ls => (vec!["ext".to_owned(), "ls".to_owned()], Vec::new()),
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum ReviewCommand {
    #[command(alias = "list")]
    #[command(about = "List recorded source-definition review stamps")]
    Ls,
    #[command(about = "Show review stamps for one package")]
    Info(super::common::PackageArg),
    #[command(about = "Forget one recorded review stamp")]
    Forget(super::common::PackageArg),
    #[command(about = "Open the reviewed recipe in a pager and compare review memory")]
    Diff(super::common::PackageArg),
}

impl ReviewCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Ls => (vec!["review".to_owned(), "ls".to_owned()], Vec::new()),
            Self::Info(args) => (
                vec!["review".to_owned(), "info".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Forget(args) => {
                let mut operands = vec![args.package.clone()];
                operands.push("--kind".to_owned());
                operands.push("interbuild".to_owned());
                (vec!["review".to_owned(), "forget".to_owned()], operands)
            }
            Self::Diff(args) => {
                let mut operands = vec![args.package.clone()];
                operands.push("--kind".to_owned());
                operands.push("interbuild".to_owned());
                (vec!["review".to_owned(), "diff".to_owned()], operands)
            }
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum TriggerCommand {
    #[command(alias = "list")]
    #[command(about = "List pending and last-run system triggers")]
    Ls,
    #[command(about = "Inspect one system trigger")]
    Info(ValueArg),
    #[command(about = "Run one pending system trigger")]
    Run(ValueArg),
    #[command(about = "Compare pending vs last-run trigger state")]
    Diff(ValueArg),
}

impl TriggerCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Ls => (vec!["trigger".to_owned(), "ls".to_owned()], Vec::new()),
            Self::Info(args) => (
                vec!["trigger".to_owned(), "info".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Run(args) => (
                vec!["trigger".to_owned(), "run".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Diff(args) => (
                vec!["trigger".to_owned(), "diff".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum MaintCommand {
    #[command(about = "Check recovery, triggers, remotes, and world health")]
    Check,
    #[command(about = "Run maintenance repair modules (recovery, triggers, profile)")]
    Fix {
        #[arg(
            value_name = "MODULE",
            help = "recovery | triggers | profile | all (default: all)"
        )]
        module: Option<String>,
    },
}

impl MaintCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Check => (vec!["maint".to_owned(), "check".to_owned()], Vec::new()),
            Self::Fix { module } => (
                vec!["maint".to_owned(), "fix".to_owned()],
                module.clone().into_iter().collect(),
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum ConfigCommand {
    #[command(about = "List pending .eldanew and .eldasave configuration files")]
    Pending,
    #[command(about = "Show a pending configuration sidecar diff")]
    Diff(ValueArg),
    #[command(about = "Apply a pending .eldanew or .eldasave sidecar")]
    Apply(ValueArg),
    #[command(about = "Keep the live file and discard the pending sidecar")]
    Keep(ValueArg),
}

impl ConfigCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Pending => (vec!["config".to_owned(), "pending".to_owned()], Vec::new()),
            Self::Diff(args) => (
                vec!["config".to_owned(), "diff".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Apply(args) => (
                vec!["config".to_owned(), "apply".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Keep(args) => (
                vec!["config".to_owned(), "keep".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum QaCommand {
    #[command(about = "Lint a package or stack")]
    Lint(OptionalValueArg),
    #[command(about = "Build a package or stack")]
    Build(OptionalValueArg),
    #[command(about = "Run smoke tests for a package or stack")]
    Smoke(OptionalValueArg),
    #[command(about = "Inspect a composed stack")]
    Stack(OptionalValueArg),
    #[command(about = "Reproduce a build or state transition")]
    Repro(OptionalValueArg),
    #[command(about = "Diff QA outputs")]
    Diff(OptionalValueArg),
}

impl QaCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
