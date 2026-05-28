use clap::{Args, Subcommand};

use super::common::{
    OptionalValueArg, PackageArg, TargetsArgs, ValueArg, VendorAddArgs, VendorExportArgs,
    VendorImportArgs, path_to_string, push_flag, push_optional,
};

#[derive(Debug, Subcommand)]
pub(super) enum RemoteCommand {
    Add(RemoteAddArgs),
    #[command(
        name = "add-from-bundle",
        about = "Register remotes from a client bundle file"
    )]
    AddFromBundle(RemoteBundleArgs),
    #[command(alias = "list")]
    Ls,
    Info(ValueArg),
    Preview(ValueArg),
    Trust(ValueArg),
    Enable(ValueArg),
    Disable(ValueArg),
    #[command(name = "set-priority", about = "Set remote priority; lower wins")]
    SetPriority(RemoteSetPriorityArgs),
    #[command(alias = "remove")]
    Rm(ValueArg),
}

impl RemoteCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => {
                let mut operands = vec![args.value.clone()];
                push_optional(
                    &mut operands,
                    "--priority",
                    args.priority.as_ref().map(ToString::to_string).as_deref(),
                );
                push_optional(&mut operands, "--trust", args.trust.as_deref());
                for trusted_key in &args.trusted_keys {
                    operands.push("--trusted-key".to_owned());
                    operands.push(trusted_key.clone());
                }
                push_optional(
                    &mut operands,
                    "--trusted-key-file",
                    args.trusted_key_file.as_deref(),
                );
                push_optional(
                    &mut operands,
                    "--signature-url",
                    args.signature_url.as_deref(),
                );
                push_optional(
                    &mut operands,
                    "--metadata-url",
                    args.metadata_url.as_deref(),
                );
                push_optional(
                    &mut operands,
                    "--packages-url",
                    args.packages_url.as_deref(),
                );
                push_optional(&mut operands, "--channel", args.channel.as_deref());
                push_flag(&mut operands, "--allow-stale", args.allow_stale);
                push_flag(&mut operands, "--replace", args.replace);
                if !args.exclude.is_empty() {
                    operands.push("--exclude".to_owned());
                    operands.extend(args.exclude.iter().cloned());
                }
                (vec!["rmt".to_owned(), "add".to_owned()], operands)
            }
            Self::AddFromBundle(args) => {
                let mut operands = vec![args.path.clone()];
                push_flag(&mut operands, "--replace", args.replace);
                (
                    vec!["rmt".to_owned(), "add-from-bundle".to_owned()],
                    operands,
                )
            }
            Self::Ls => (vec!["rmt".to_owned(), "ls".to_owned()], Vec::new()),
            Self::Info(args) => (
                vec!["rmt".to_owned(), "info".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Preview(args) => (
                vec!["rmt".to_owned(), "preview".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Trust(args) => (
                vec!["rmt".to_owned(), "trust".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Enable(args) => (
                vec!["rmt".to_owned(), "enable".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Disable(args) => (
                vec!["rmt".to_owned(), "disable".to_owned()],
                vec![args.value.clone()],
            ),
            Self::SetPriority(args) => (
                vec!["rmt".to_owned(), "set-priority".to_owned()],
                vec![args.name.clone(), args.priority.to_string()],
            ),
            Self::Rm(args) => (
                vec!["rmt".to_owned(), "rm".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct RemoteSetPriorityArgs {
    #[arg(help = "Remote name")]
    name: String,
    #[arg(help = "Remote priority; lower wins")]
    priority: u32,
}

#[derive(Debug, Args)]
pub(super) struct RemoteBundleArgs {
    #[arg(help = "Client bundle TOML or JSON path")]
    path: String,
    #[arg(long, help = "Replace existing remotes with the same name")]
    replace: bool,
}

#[derive(Debug, Args)]
pub(super) struct RemoteAddArgs {
    #[arg(help = "Remote index URL or <name>=<url>")]
    value: String,
    #[arg(long, help = "Remote priority; lower wins")]
    priority: Option<u32>,
    #[arg(long, help = "Trust mode: tofu, pinned, or insecure")]
    trust: Option<String>,
    #[arg(
        long = "trusted-key",
        help = "Pinned key id or fingerprint",
        value_name = "KEY"
    )]
    trusted_keys: Vec<String>,
    #[arg(long = "trusted-key-file", help = "Path to pinned keys file")]
    trusted_key_file: Option<String>,
    #[arg(long = "signature-url", help = "Detached signature URL override")]
    signature_url: Option<String>,
    #[arg(
        long = "metadata-url",
        help = "Signed remote metadata / rotation document URL"
    )]
    metadata_url: Option<String>,
    #[arg(
        long = "packages-url",
        help = "Package-definition git URL for source remotes"
    )]
    packages_url: Option<String>,
    #[arg(long, help = "Remote channel to sync")]
    channel: Option<String>,
    #[arg(
        long = "allow-stale",
        help = "Allow stale verified snapshots when refresh fails"
    )]
    allow_stale: bool,
    #[arg(long, help = "Replace an existing remote document with the same name")]
    replace: bool,
    #[arg(
        long,
        num_args = 1..,
        value_name = "PKG",
        help = "Exclude packages from interemote sync; place after other flags. Values may be comma-separated; more tokens list more packages"
    )]
    exclude: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub(super) enum RecipeCommand {
    Add(RecipeAddArgs),
    Show(PackageArg),
    Diff(PackageArg),
    #[command(name = "publish-ready")]
    PublishReady(PackageArg),
    Edit(PackageArg),
    Check(RecipeCheckArgs),
    Format(PackageArg),
    Normalize(PackageArg),
    #[command(alias = "list")]
    Ls,
    Rm(PackageArg),
}

impl RecipeCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => {
                let mut operands = vec![args.value.clone()];
                push_optional(&mut operands, "--kind", args.kind.as_deref());
                push_flag(&mut operands, "--replace", args.replace);
                (vec!["rc".to_owned(), "add".to_owned()], operands)
            }
            Self::Show(args) => (
                vec!["rc".to_owned(), "show".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Diff(args) => (
                vec!["rc".to_owned(), "diff".to_owned()],
                vec![args.package.clone()],
            ),
            Self::PublishReady(args) => (
                vec!["rc".to_owned(), "publish-ready".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Edit(args) => (
                vec!["rc".to_owned(), "edit".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Check(args) => {
                let mut operands = args.package.clone().into_iter().collect::<Vec<_>>();
                push_flag(&mut operands, "--strict", args.strict);
                (vec!["rc".to_owned(), "check".to_owned()], operands)
            }
            Self::Format(args) => (
                vec!["rc".to_owned(), "format".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Normalize(args) => (
                vec!["rc".to_owned(), "normalize".to_owned()],
                vec![args.package.clone()],
            ),
            Self::Ls => (vec!["rc".to_owned(), "ls".to_owned()], Vec::new()),
            Self::Rm(args) => (
                vec!["rc".to_owned(), "rm".to_owned()],
                vec![args.package.clone()],
            ),
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct RecipeCheckArgs {
    #[arg(value_name = "PACKAGE", help = "Optional local recipe package name")]
    package: Option<String>,
    #[arg(long, help = "Treat warnings as failures")]
    strict: bool,
}

#[derive(Debug, Args)]
pub(super) struct RecipeAddArgs {
    #[arg(help = "Package name, git source, or local path")]
    value: String,
    #[arg(long, help = "Scaffold recipe kind: normal, meta, or profile")]
    kind: Option<String>,
    #[arg(long, help = "Replace existing local recipe metadata")]
    replace: bool,
}

#[derive(Debug, Subcommand)]
pub(super) enum CiCommand {
    #[command(about = "Submit package work for CI publishing")]
    Sub(TargetsArgs),
    #[command(about = "Run CI work locally or through the configured slice")]
    Run(TargetsArgs),
    #[command(about = "Inspect CI status")]
    Status(OptionalValueArg),
    #[command(about = "Open or inspect the current CI pull request")]
    Pr(OptionalValueArg),
    #[command(about = "Retry failed CI work")]
    Retry(OptionalValueArg),
    #[command(about = "Inspect CI logs")]
    Logs(OptionalValueArg),
    #[command(about = "Manage CI batches")]
    Batch {
        #[command(subcommand)]
        command: BatchCommand,
    },
}

impl CiCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
pub(super) enum BatchCommand {
    #[command(about = "Create a new batch definition")]
    New(OptionalValueArg),
    #[command(about = "Add packages to the active batch")]
    Add(TargetsArgs),
    #[command(about = "Push the active batch for processing")]
    Push(OptionalValueArg),
}

impl BatchCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
pub(super) enum VendorCommand {
    #[command(about = "Create or update a local vendor recipe")]
    Add(VendorAddArgs),
    #[command(about = "Import vendor recipes from a manifest or lock file")]
    Import(VendorImportArgs),
    #[command(about = "Export vendor recipes into a manifest or lock file")]
    Export(VendorExportArgs),
}

impl VendorCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Add(args) => {
                let mut operands = vec![args.package.clone(), args.source.clone()];
                push_optional(&mut operands, "--binary", args.binary.as_deref());
                push_optional(&mut operands, "--asset", args.asset.as_deref());
                push_flag(&mut operands, "--replace", args.replace);
                (vec!["vendor".to_owned(), "add".to_owned()], operands)
            }
            Self::Import(args) => {
                let mut operands = vec![path_to_string(&args.path)];
                push_flag(&mut operands, "--replace", args.replace);
                (vec!["vendor".to_owned(), "import".to_owned()], operands)
            }
            Self::Export(args) => {
                let mut operands = vec![path_to_string(&args.path)];
                operands.extend(args.packages.clone());
                (vec!["vendor".to_owned(), "export".to_owned()], operands)
            }
        }
    }
}

#[derive(Debug, Subcommand)]
pub(super) enum ForgeCommand {
    #[command(about = "Search forge metadata outside the solver")]
    Search(ValueArg),
    #[command(about = "Browse one forge package or repository")]
    Browse(ValueArg),
    #[command(about = "Fork a forge repository through the GitHub CLI")]
    Fork(ValueArg),
}

impl ForgeCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Search(args) => (
                vec!["forge".to_owned(), "search".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Browse(args) => (
                vec!["forge".to_owned(), "browse".to_owned()],
                vec![args.value.clone()],
            ),
            Self::Fork(args) => (
                vec!["forge".to_owned(), "fork".to_owned()],
                vec![args.value.clone()],
            ),
        }
    }
}
