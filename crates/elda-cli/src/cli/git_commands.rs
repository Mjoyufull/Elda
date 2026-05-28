use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub(super) enum GitCommand {
    #[command(about = "List and normalize remote git tags")]
    Tags(GitTagsArgs),
    #[command(about = "Inspect forge release assets for the current host")]
    Releases(GitReleasesArgs),
}

impl GitCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Tags(args) => {
                let mut operands = vec![args.target.clone()];
                if let Some(max_tags) = args.max_tags {
                    operands.push("--max-tags".to_owned());
                    operands.push(max_tags.to_string());
                }
                if args.with_releases {
                    operands.push("--with-releases".to_owned());
                }
                (vec!["git".to_owned(), "tags".to_owned()], operands)
            }
            Self::Releases(args) => {
                let mut operands = vec![args.target.clone()];
                if let Some(max_releases) = args.max_releases {
                    operands.push("--max-releases".to_owned());
                    operands.push(max_releases.to_string());
                }
                if let Some(tag) = &args.tag {
                    operands.push("--tag".to_owned());
                    operands.push(tag.clone());
                }
                (vec!["git".to_owned(), "releases".to_owned()], operands)
            }
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct GitTagsArgs {
    pub(super) target: String,
    #[arg(long = "max-tags", value_name = "N")]
    pub(super) max_tags: Option<usize>,
    #[arg(
        long = "with-releases",
        help = "Join recent forge releases to matching tags"
    )]
    pub(super) with_releases: bool,
}

#[derive(Debug, Args)]
pub(super) struct GitReleasesArgs {
    target: String,
    #[arg(long = "max-releases", value_name = "N")]
    max_releases: Option<usize>,
    #[arg(long = "tag", value_name = "REF", help = "Show only one release tag")]
    tag: Option<String>,
}
