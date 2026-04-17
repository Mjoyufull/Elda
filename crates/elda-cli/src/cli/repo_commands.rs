use clap::{Args, Subcommand};

use super::common::{
    OptionalPackageArg, OptionalValueArg, PackageArg, PathArg, TargetsArgs, ValueArg,
    VendorAddArgs, VendorExportArgs, VendorImportArgs, path_to_string, push_flag, push_optional,
};

#[derive(Debug, Subcommand)]
pub(super) enum RemoteCommand {
    #[command(about = "Register a remote index document")]
    Add(RemoteAddArgs),
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
                push_flag(&mut operands, "--allow-stale", args.allow_stale);
                (vec!["rmt".to_owned(), "add".to_owned()], operands)
            }
        }
    }
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
        long = "allow-stale",
        help = "Allow stale verified snapshots when refresh fails"
    )]
    allow_stale: bool,
}

#[derive(Debug, Subcommand)]
pub(super) enum RecipeCommand {
    #[command(about = "Add or import a local recipe")]
    Add(PathArg),
    #[command(about = "Open an existing local recipe")]
    Edit(PackageArg),
    #[command(about = "Validate local recipes")]
    Check(OptionalPackageArg),
}

impl RecipeCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
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
                (vec!["vendor".to_owned(), "add".to_owned()], operands)
            }
            Self::Import(args) => (
                vec!["vendor".to_owned(), "import".to_owned()],
                vec![path_to_string(&args.path)],
            ),
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
        }
    }
}
