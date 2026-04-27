#![forbid(unsafe_code)]

mod cli;
mod help;
mod privilege;

use clap::{CommandFactory, Parser};
use cli::Cli;
use elda_core::{CommandRequest, CoreError, render_human, run};
use elda_types::OutputMode;
use privilege::reexec_with_provider;

fn main() {
    if let Err(error) = try_main() {
        eprintln!("Error: {error}");
        let root_message = error.to_string();
        let causes = error
            .chain()
            .skip(1)
            .map(ToString::to_string)
            .filter(|cause| cause != &root_message)
            .collect::<Vec<_>>();
        if !causes.is_empty() {
            eprintln!();
            eprintln!("Caused by:");
            for (index, cause) in causes.iter().enumerate() {
                eprintln!("    {}: {cause}", index);
            }
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

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if let Some(request) = fallback_bare_query_request(&raw_args) {
                return execute_request(request);
            }
            return Err(error.into());
        }
    };

    let Some(request) = cli.command_request() else {
        let mut command = Cli::command();
        command.print_long_help()?;
        println!();
        return Ok(());
    };

    execute_request(request)
}

fn execute_request(request: CommandRequest) -> anyhow::Result<()> {
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

fn fallback_bare_query_request(raw_args: &[std::ffi::OsString]) -> Option<CommandRequest> {
    if raw_args.len() != 2 {
        return None;
    }
    let query = raw_args[1].to_string_lossy().to_string();
    if query.trim().is_empty() || query.starts_with('-') {
        return None;
    }

    Some(CommandRequest::new(
        vec!["search".to_owned()],
        vec![query, "--interactive".to_owned()],
        OutputMode::Human,
        false,
    ))
}
