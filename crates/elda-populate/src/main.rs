#![forbid(unsafe_code)]

mod cache;
mod cli;
mod config;
mod error;
mod manifest;
mod operations;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();
    match operations::run(cli) {
        Ok(report) => println!("{}", report.summary),
        Err(error) => {
            eprintln!("populate: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests;
