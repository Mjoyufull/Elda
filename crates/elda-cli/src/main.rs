#![forbid(unsafe_code)]

mod cli;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::Cli;
use elda_core::run;
use elda_types::OutputMode;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let Some(request) = cli.command_request() else {
        let mut command = Cli::command();
        command.print_long_help()?;
        println!();
        return Ok(());
    };

    let report = run(request);
    match report.output_mode {
        OutputMode::Human => println!("{}", report.summary),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(())
}
