#![forbid(unsafe_code)]

mod clean;
mod cli;
mod commands;

use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    ExitCode::from(commands::run(cli) as u8)
}
