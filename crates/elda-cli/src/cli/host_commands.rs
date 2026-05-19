use clap::{Args, Subcommand};

use super::common::{OptionalValueArg, ValueArg, push_flag, push_optional};

#[derive(Debug, Subcommand)]
pub(super) enum HostCommand {
    #[command(about = "Scan a recipe tree for parse and publish-ready status")]
    ScanTree(HostTreeArgs),
    #[command(about = "Validate and plan a recipe tree (dry-run by default)")]
    TestTree(HostTreeArgs),
    #[command(about = "List packages changed in a recipe tree since a git ref")]
    DiffTree(HostDiffTreeArgs),
    #[command(about = "Push recipe tree commits to the configured forge remote")]
    PushRecipes(HostProfileArgs),
    #[command(about = "Emit client remote registration snippets for a host profile")]
    ClientBundle(OptionalValueArg),
    #[command(about = "Write a GitHub/GitLab Elda publish workflow template")]
    InitCi(HostInitCiArgs),
    #[command(about = "Check host profile paths, signing, and publish settings")]
    Doctor(HostProfileArgs),
    #[command(about = "Show last publish artifacts per channel")]
    Status(HostProfileArgs),
    #[command(about = "Sync a maintainer recipe tree into the CI workspace")]
    Link(ValueArg),
    #[command(about = "Print example static cache front-end configuration")]
    PrintCacheConfig(ValueArg),
}

impl HostCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::ScanTree(args) => host_tree_parts("scan-tree", args),
            Self::TestTree(args) => host_tree_parts("test-tree", args),
            Self::DiffTree(args) => {
                let mut operands = args.tree.clone().into_iter().collect::<Vec<_>>();
                operands.push("--since".to_owned());
                operands.push(args.since.clone());
                push_optional(&mut operands, "--profile", args.profile.as_deref());
                (vec!["host".to_owned(), "diff-tree".to_owned()], operands)
            }
            Self::PushRecipes(args) => host_profile_parts("push-recipes", args),
            Self::ClientBundle(args) => (
                vec!["host".to_owned(), "client-bundle".to_owned()],
                args.value.clone().into_iter().collect(),
            ),
            Self::Doctor(args) => host_profile_parts("doctor", args),
            Self::Status(args) => host_profile_parts("status", args),
            Self::Link(args) => (
                vec!["host".to_owned(), "link".to_owned()],
                vec![args.value.clone()],
            ),
            Self::PrintCacheConfig(args) => (
                vec!["host".to_owned(), "print-cache-config".to_owned()],
                vec![args.value.clone()],
            ),
            Self::InitCi(args) => {
                let mut operands = Vec::new();
                push_optional(&mut operands, "--forge", args.forge.as_deref());
                push_flag(&mut operands, "--force", args.force);
                (vec!["host".to_owned(), "init-ci".to_owned()], operands)
            }
        }
    }
}

fn host_tree_parts(command: &str, args: &HostTreeArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.tree.clone().into_iter().collect::<Vec<_>>();
    push_optional(&mut operands, "--profile", args.profile.as_deref());
    push_flag(&mut operands, "--install", args.install);
    for package in &args.only {
        operands.push("--only".to_owned());
        operands.push(package.clone());
    }
    (vec!["host".to_owned(), command.to_owned()], operands)
}

fn host_profile_parts(command: &str, args: &HostProfileArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = Vec::new();
    push_optional(&mut operands, "--profile", args.profile.as_deref());
    (vec!["host".to_owned(), command.to_owned()], operands)
}

#[derive(Debug, Args)]
pub(super) struct HostTreeArgs {
    #[arg(value_name = "PATH", help = "Recipe monorepo root")]
    tree: Option<String>,
    #[arg(long, help = "Host profile name from etc/elda/host.d/")]
    profile: Option<String>,
    #[arg(long, help = "Run disposable-root install smoke tests")]
    install: bool,
    #[arg(long = "only", value_name = "PKG", help = "Limit to specific packages")]
    only: Vec<String>,
}

#[derive(Debug, Args)]
pub(super) struct HostDiffTreeArgs {
    #[arg(value_name = "PATH", help = "Recipe monorepo root")]
    tree: Option<String>,
    #[arg(long, help = "Git ref to diff against")]
    since: String,
    #[arg(long, help = "Host profile name from etc/elda/host.d/")]
    profile: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct HostProfileArgs {
    #[arg(long, help = "Host profile name from etc/elda/host.d/")]
    profile: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct HostInitCiArgs {
    #[arg(long, help = "Forge template flavor: github, gitlab, or gitea")]
    forge: Option<String>,
    #[arg(long, help = "Overwrite an existing workflow file")]
    force: bool,
}
