pub mod baseline;
pub mod cli;
pub mod compare;
pub mod find_limit;
pub mod hit;
pub mod output;
pub mod ramp;
pub mod report;
pub mod request;
pub mod runner;
pub mod scenario;
pub mod spike;
pub mod stats;

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{Cli, Commands};

pub async fn run_cli(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Commands::Hit(args) => hit::run(args).await,
        Commands::Ramp(args) => ramp::run(args).await,
        Commands::Spike(args) => spike::run(args).await,
        Commands::FindLimit(args) => find_limit::run(args).await,
        Commands::Compare(args) => Ok(compare::run(args)?),
        Commands::Scenario(args) => scenario::run(args).await,
    }
}
