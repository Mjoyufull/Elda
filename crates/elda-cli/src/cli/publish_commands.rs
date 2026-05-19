use clap::{Args, Subcommand};

use super::common::{TargetsArgs, push_flag, push_optional};

#[derive(Debug, Subcommand)]
pub(super) enum PublishCommand {
    #[command(about = "Plan publish work for packages or a recipe tree")]
    Plan(PublishScopeArgs),
    #[command(about = "Build packages and write a local signed index")]
    Run(PublishScopeArgs),
    #[command(about = "Rewrite file:// index URLs to HTTPS and emit index-v1.json.zst")]
    Finalize(PublishFinalizeArgs),
    #[command(about = "Compare the current channel index to a previous index file")]
    Diff(PublishDiffArgs),
    #[command(about = "Promote artifacts and index rows between channels")]
    Promote(PublishPromoteArgs),
    #[command(about = "Re-sign the current channel index")]
    Sign(PublishSignArgs),
}

impl PublishCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Plan(args) => publish_scope_parts("plan", args),
            Self::Run(args) => publish_scope_parts("run", args),
            Self::Finalize(args) => {
                let mut operands = Vec::new();
                push_optional(&mut operands, "--channel", args.channel.as_deref());
                push_optional(&mut operands, "--base-url", args.base_url.as_deref());
                push_optional(&mut operands, "--profile", args.profile.as_deref());
                (vec!["publish".to_owned(), "finalize".to_owned()], operands)
            }
            Self::Diff(args) => {
                let mut operands = args.previous.clone().into_iter().collect::<Vec<_>>();
                push_optional(&mut operands, "--channel", args.channel.as_deref());
                (vec!["publish".to_owned(), "diff".to_owned()], operands)
            }
            Self::Promote(args) => {
                let mut operands = Vec::new();
                push_optional(&mut operands, "--from", args.from.as_deref());
                push_optional(&mut operands, "--to", args.to.as_deref());
                (vec!["publish".to_owned(), "promote".to_owned()], operands)
            }
            Self::Sign(args) => {
                let mut operands = Vec::new();
                push_optional(&mut operands, "--channel", args.channel.as_deref());
                push_optional(&mut operands, "--key", args.key.as_deref());
                push_optional(&mut operands, "--profile", args.profile.as_deref());
                (vec!["publish".to_owned(), "sign".to_owned()], operands)
            }
        }
    }
}

fn publish_scope_parts(command: &str, args: &PublishScopeArgs) -> (Vec<String>, Vec<String>) {
    let mut operands = args.targets.targets.clone();
    push_optional(&mut operands, "--tree", args.tree.as_deref());
    push_optional(&mut operands, "--channel", args.channel.as_deref());
    push_optional(&mut operands, "--profile", args.profile.as_deref());
    (vec!["publish".to_owned(), command.to_owned()], operands)
}

#[derive(Debug, Args)]
pub(super) struct PublishScopeArgs {
    #[command(flatten)]
    targets: TargetsArgs,
    #[arg(long, help = "Recipe monorepo root containing packages/")]
    tree: Option<String>,
    #[arg(long, help = "Publish channel name")]
    channel: Option<String>,
    #[arg(long, help = "Host profile for signing and defaults")]
    profile: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct PublishFinalizeArgs {
    #[arg(long, help = "HTTPS base URL for finalized asset and index links")]
    base_url: Option<String>,
    #[arg(long, help = "Channel whose published index should be finalized")]
    channel: Option<String>,
    #[arg(long, help = "Host profile providing base_url when omitted")]
    profile: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct PublishDiffArgs {
    #[arg(
        value_name = "INDEX",
        help = "Optional previous index JSON or .zst path"
    )]
    previous: Option<String>,
    #[arg(long, help = "Channel whose current index should be compared")]
    channel: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct PublishPromoteArgs {
    #[arg(long, help = "Source channel to promote from")]
    from: Option<String>,
    #[arg(long, help = "Destination channel to promote into")]
    to: Option<String>,
}

#[derive(Debug, Args)]
pub(super) struct PublishSignArgs {
    #[arg(long, help = "Channel index to re-sign")]
    channel: Option<String>,
    #[arg(long, help = "Signing key path override")]
    key: Option<String>,
    #[arg(long, help = "Host profile for signing key lookup")]
    profile: Option<String>,
}
