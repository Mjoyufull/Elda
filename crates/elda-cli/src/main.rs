#![forbid(unsafe_code)]

mod cli;
mod help;
mod privilege;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use cli::Cli;
use elda_core::{
    CommandRequest, CoreError, process_exit_code, render_human, report_frontend_failure,
    report_runtime_failure, run,
};
use elda_types::OutputMode;
use privilege::reexec_with_provider;

fn main() {
    if let Err(error) = try_main() {
        let blocked = error.to_string();
        let causes = error
            .chain()
            .skip(1)
            .map(ToString::to_string)
            .filter(|cause| cause != &blocked)
            .collect::<Vec<_>>();
        let report = report_frontend_failure(blocked, causes);
        eprintln!("{}", render_human(&report));
        std::process::exit(process_exit_code(report.exit_status));
    }
}

fn try_main() -> anyhow::Result<()> {
    let raw_args = std::env::args_os().collect::<Vec<_>>();
    if help::should_print_root_help(&raw_args) {
        help::print_root_help();
        return Ok(());
    }

    if help::should_print_version(&raw_args) {
        println!("{}", elda_core::cli_long_version());
        return Ok(());
    }

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if is_help_or_version_error(&error) {
                let framed = error.to_string();
                if framed.contains("Usage:") {
                    help::print_framed_subcommand_help("elda", &framed);
                } else {
                    print!("{error}");
                }
                return Ok(());
            }
            if let Some(request) = fallback_bare_query_request(&raw_args) {
                return execute_request(request);
            }
            return Err(error.into());
        }
    };

    let Some(request) = cli.command_request() else {
        let mut command = Cli::command();
        let mut help = Vec::new();
        command.write_long_help(&mut help)?;
        help::print_framed_subcommand_help("elda", std::str::from_utf8(&help).unwrap_or_default());
        println!();
        return Ok(());
    };

    execute_request(request)
}

fn is_help_or_version_error(error: &clap::Error) -> bool {
    matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    )
}

fn execute_request(request: CommandRequest) -> anyhow::Result<()> {
    let report = match run(request.clone()) {
        Ok(report) => report,
        Err(CoreError::PrivilegeRequired(request)) => return reexec_with_provider(&request),
        Err(error) => report_runtime_failure(&error, &request),
    };
    let exit_status = report.exit_status;
    match report.output_mode {
        OutputMode::Human => println!("{}", render_human(&report)),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
    }

    if exit_status == elda_types::ExitStatus::Success {
        Ok(())
    } else {
        std::process::exit(process_exit_code(exit_status));
    }
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, is_help_or_version_error};

    #[test]
    fn command_specific_help_is_not_a_frontend_failure() {
        let error = Cli::try_parse_from(["elda", "rmt", "add", "--help"])
            .expect_err("help should be represented as a clap display error");

        assert!(is_help_or_version_error(&error));
        assert!(error.to_string().contains("Usage: elda rmt add"));
    }

    #[test]
    fn parse_errors_still_flow_to_frontend_failure() {
        let error = Cli::try_parse_from(["elda", "rmt", "add"])
            .expect_err("missing required argument should fail");

        assert!(!is_help_or_version_error(&error));
    }
}
