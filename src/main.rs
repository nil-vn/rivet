//! FurrumX executable entry point.

use std::process::ExitCode;

use clap::Parser;
use furrumx::cli::Cli;
use furrumx::runtime::init_telemetry;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(error) = init_telemetry() {
        eprintln!("{error}");
        return ExitCode::FAILURE;
    }

    cli.execute()
}
