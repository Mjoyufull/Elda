#![forbid(unsafe_code)]

mod cli;
mod help;
mod privilege;

use clap::{CommandFactory, Parser};
use cli::Cli;
use elda_core::{CoreError, render_human, run};
use elda_types::OutputMode;
use privilege::reexec_with_provider;

fn main() {
    if let Err(error) = try_main() {
        eprintln!("Error: {error}");
        for (index, cause) in error.chain().skip(1).enumerate() {
            if index == 0 {
                eprintln!();
                eprintln!("Caused by:");
            }
            eprintln!("    {}: {cause}", index);
        }
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    if help::should_print_root_help(&raw_args) {
        help::print_root_help();
        return Ok(());
    }

    let cli = Cli::parse();

    let Some(request) = cli.command_request() else {
        let mut command = Cli::command();
        command.print_long_help()?;
        println!();
        return Ok(());
    };

    let report = match run(request) {
        Ok(report) => report,
        Err(CoreError::PrivilegeRequired(request)) => return reexec_with_provider(&request),
        Err(error) => return Err(error.into()),
    };
    match report.output_mode {
        OutputMode::Human => println!("{}", render_human(&report)),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    Ok(())
}
