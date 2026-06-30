use std::process::ExitCode;

use clap::Parser;

use adrenaline::cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    match adrenaline::run_cli(cli).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
