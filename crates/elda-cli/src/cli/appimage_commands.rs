use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::common::path_to_string;

#[derive(Debug, Subcommand)]
pub(super) enum AppImageCommand {
    #[command(
        about = "Inspect a Type 2 AppImage SquashFS payload without executing it (desktop, icons, metainfo)"
    )]
    Inspect(AppImageInspectArgs),
}

impl AppImageCommand {
    pub(super) fn request_parts(&self) -> (Vec<String>, Vec<String>) {
        match self {
            Self::Inspect(args) => (
                vec!["appimage".to_owned(), "inspect".to_owned()],
                vec![path_to_string(&args.path)],
            ),
        }
    }
}

#[derive(Debug, Args)]
pub(super) struct AppImageInspectArgs {
    pub(super) path: PathBuf,
}
