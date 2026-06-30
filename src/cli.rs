use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, Parser, Subcommand};

use crate::request::HttpMethod;

#[derive(Parser, Debug)]
#[command(name = "adrenaline")]
#[command(version)]
#[command(about = "A lightweight, fast API stress tester")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Hit(HitArgs),
    Ramp(RampArgs),
    Spike(SpikeArgs),
    FindLimit(FindLimitArgs),
    Compare(CompareArgs),
    Scenario(ScenarioArgs),
}

#[derive(Args, Debug, Clone)]
pub struct OutputArgs {
    #[arg(long, help = "Print results as JSON")]
    pub json: bool,

    #[arg(long, value_name = "FILE", help = "Write HTML report to FILE")]
    pub html: Option<PathBuf>,

    #[arg(
        long,
        value_name = "FILE",
        help = "Compare results against a saved baseline"
    )]
    pub baseline: Option<PathBuf>,

    #[arg(long, value_name = "FILE", help = "Save results as baseline JSON")]
    pub save_baseline: Option<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct RequestArgs {
    pub url: String,

    #[arg(long, default_value = "GET")]
    pub method: HttpMethod,

    #[arg(long = "header", value_name = "KEY: VALUE")]
    pub headers: Vec<String>,

    #[arg(long, value_name = "FILE")]
    pub body: Option<PathBuf>,

    #[arg(long, default_value_t = 10)]
    pub timeout: u64,
}

#[derive(Args, Debug, Clone)]
pub struct HitArgs {
    #[command(flatten)]
    pub request: RequestArgs,

    #[arg(short = 'n', long = "requests", default_value_t = 1000)]
    pub requests: usize,

    #[arg(short = 'c', long = "concurrency", default_value_t = 50)]
    pub concurrency: usize,

    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Args, Debug, Clone)]
pub struct RampArgs {
    #[command(flatten)]
    pub request: RequestArgs,

    #[arg(short = 'n', long = "requests", default_value_t = 1000)]
    pub requests: usize,

    #[arg(long, default_value_t = 1)]
    pub start_concurrency: usize,

    #[arg(long, default_value_t = 50)]
    pub end_concurrency: usize,

    #[arg(long, default_value_t = 10)]
    pub steps: usize,

    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Args, Debug, Clone)]
pub struct SpikeArgs {
    #[command(flatten)]
    pub request: RequestArgs,

    #[arg(long, default_value_t = 100)]
    pub baseline_requests: usize,

    #[arg(long, default_value_t = 10)]
    pub baseline_concurrency: usize,

    #[arg(long, default_value_t = 500)]
    pub spike_requests: usize,

    #[arg(long, default_value_t = 100)]
    pub spike_concurrency: usize,

    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Args, Debug, Clone)]
pub struct FindLimitArgs {
    #[command(flatten)]
    pub request: RequestArgs,

    #[arg(short = 'n', long = "requests-per-step", default_value_t = 100)]
    pub requests_per_step: usize,

    #[arg(long, default_value_t = 1)]
    pub start_concurrency: usize,

    #[arg(long, default_value_t = 500)]
    pub max_concurrency: usize,

    #[arg(long, default_value_t = 10)]
    pub step: usize,

    #[arg(long, default_value_t = 5.0)]
    pub max_error_rate: f64,

    #[command(flatten)]
    pub output: OutputArgs,
}

#[derive(Args, Debug, Clone)]
pub struct CompareArgs {
    pub baseline: PathBuf,
    pub current: PathBuf,

    #[arg(
        long,
        default_value_t = 10.0,
        help = "Allowed p99 latency regression in percent"
    )]
    pub p99_threshold: f64,

    #[arg(
        long,
        default_value_t = 1.0,
        help = "Allowed error rate regression in percent points"
    )]
    pub error_rate_threshold: f64,
}

#[derive(Args, Debug, Clone)]
pub struct ScenarioArgs {
    pub file: PathBuf,

    #[command(flatten)]
    pub output: OutputArgs,
}

pub fn validate_positive(value: usize, name: &str) -> Result<()> {
    if value == 0 {
        bail!("{name} must be greater than 0");
    }
    Ok(())
}

pub fn validate_hit_args(args: &HitArgs) -> Result<()> {
    validate_positive(args.requests, "requests")?;
    validate_positive(args.concurrency, "concurrency")?;
    Ok(())
}

pub fn validate_ramp_args(args: &RampArgs) -> Result<()> {
    validate_positive(args.requests, "requests")?;
    validate_positive(args.start_concurrency, "start concurrency")?;
    validate_positive(args.end_concurrency, "end concurrency")?;
    validate_positive(args.steps, "steps")?;
    Ok(())
}

pub fn validate_spike_args(args: &SpikeArgs) -> Result<()> {
    validate_positive(args.baseline_requests, "baseline requests")?;
    validate_positive(args.baseline_concurrency, "baseline concurrency")?;
    validate_positive(args.spike_requests, "spike requests")?;
    validate_positive(args.spike_concurrency, "spike concurrency")?;
    Ok(())
}

pub fn validate_find_limit_args(args: &FindLimitArgs) -> Result<()> {
    validate_positive(args.requests_per_step, "requests per step")?;
    validate_positive(args.start_concurrency, "start concurrency")?;
    validate_positive(args.max_concurrency, "max concurrency")?;
    validate_positive(args.step, "step")?;
    if args.max_error_rate < 0.0 || args.max_error_rate > 100.0 {
        bail!("max error rate must be between 0 and 100");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_hit_rejects_zero_requests() {
        let args = HitArgs {
            request: RequestArgs {
                url: "https://example.com".to_string(),
                method: HttpMethod::Get,
                headers: vec![],
                body: None,
                timeout: 10,
            },
            requests: 0,
            concurrency: 10,
            output: OutputArgs {
                json: false,
                html: None,
                baseline: None,
                save_baseline: None,
            },
        };
        let err = validate_hit_args(&args).unwrap_err();
        assert_eq!(err.to_string(), "requests must be greater than 0");
    }

    #[test]
    fn validate_hit_rejects_zero_concurrency() {
        let args = HitArgs {
            request: RequestArgs {
                url: "https://example.com".to_string(),
                method: HttpMethod::Get,
                headers: vec![],
                body: None,
                timeout: 10,
            },
            requests: 10,
            concurrency: 0,
            output: OutputArgs {
                json: false,
                html: None,
                baseline: None,
                save_baseline: None,
            },
        };
        let err = validate_hit_args(&args).unwrap_err();
        assert_eq!(err.to_string(), "concurrency must be greater than 0");
    }
}
