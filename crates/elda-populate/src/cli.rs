use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "populate")]
#[command(about = "Companion tooling for cache seeding and remote mirroring.")]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        help = "Elda root directory; defaults to ELDA_ROOT_DIR or `/`"
    )]
    pub(crate) root: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        help = "Prefix to inspect; defaults to config.toml or `/usr`"
    )]
    pub(crate) prefix: Option<PathBuf>,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum CacheCommand {
    PushLocal(PushLocalArgs),
    MirrorRemote(MirrorRemoteArgs),
}

#[derive(Debug, Args)]
pub(crate) struct PushLocalArgs {
    #[arg(long, help = "Configured cache name to populate")]
    pub(crate) cache: String,
    #[arg(
        long,
        help = "Push payloads referenced by currently installed packages"
    )]
    pub(crate) installed: bool,
    #[arg(
        long,
        help = "Only push payloads for the named package",
        value_name = "PKG"
    )]
    pub(crate) package: Vec<String>,
    #[arg(long, help = "Emit a cache-seed manifest JSON file")]
    pub(crate) manifest_out: Option<PathBuf>,
    #[arg(
        long,
        help = "Report what would be pushed without writing to the cache"
    )]
    pub(crate) dry_run: bool,
}

#[derive(Debug, Args)]
pub(crate) struct MirrorRemoteArgs {
    #[arg(long, help = "Configured cache name to populate")]
    pub(crate) cache: String,
    #[arg(long, help = "Synced remote name to mirror from")]
    pub(crate) remote: String,
    #[arg(long, help = "Only mirror packages from the selected channel")]
    pub(crate) channel: Option<String>,
    #[arg(long, help = "Only mirror the named package", value_name = "PKG")]
    pub(crate) package: Vec<String>,
    #[arg(long, help = "Emit a cache-seed manifest JSON file")]
    pub(crate) manifest_out: Option<PathBuf>,
    #[arg(
        long,
        help = "Report what would be mirrored without writing to the cache"
    )]
    pub(crate) dry_run: bool,
}
