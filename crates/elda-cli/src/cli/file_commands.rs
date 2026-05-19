use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::common::{ValueArg, path_to_string};

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
pub(super) struct FilesArgs {
    #[arg(value_name = "PKG")]
    pub(super) package: Option<String>,
    #[command(subcommand)]
    pub(super) command: Option<FilesSubcommand>,
}

#[derive(Debug, Subcommand)]
pub(super) enum FilesSubcommand {
    #[command(about = "Search installed managed paths")]
    Search(ValueArg),
    #[command(about = "Show which installed package owns a managed path")]
    Owner(FilesOwnerArgs),
}

#[derive(Debug, Args)]
pub(super) struct FilesOwnerArgs {
    #[arg(help = "Absolute managed path")]
    pub(super) path: PathBuf,
}

pub(super) fn request_parts(args: &FilesArgs) -> (Vec<String>, Vec<String>) {
    if let Some(subcommand) = &args.command {
        match subcommand {
            FilesSubcommand::Search(search) => (
                vec!["files".to_owned(), "search".to_owned()],
                vec![search.value.clone()],
            ),
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
