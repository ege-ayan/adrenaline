use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{RampArgs, validate_ramp_args};
use crate::output::request_spec_from_args;
use crate::report::{finish_report, metadata_map};
use crate::request::build_client;
use crate::runner::execute_load;
use crate::stats::TestReport;

pub async fn run(args: RampArgs) -> Result<ExitCode> {
    validate_ramp_args(&args)?;

    let spec = request_spec_from_args(&args.request).await?;
    let client = build_client(spec.timeout_secs).await?;

    let requests_per_step = args.requests / args.steps;
    let remainder = args.requests % args.steps;

    let mut reports = Vec::new();
    let mut total_duration = std::time::Duration::ZERO;

    for step in 0..args.steps {
        let step_requests = requests_per_step + if step == args.steps - 1 { remainder } else { 0 };
        if step_requests == 0 {
            continue;
        }

        let concurrency = ramp_concurrency(
            args.start_concurrency,
            args.end_concurrency,
            step,
            args.steps,
        );

        let (stats, duration) =
            execute_load(&client, &spec, step_requests, concurrency, None).await?;

        let report = stats.finalize(
            "ramp",
            &spec.url,
            spec.method.as_str(),
            duration,
            step_requests,
            metadata_map(&[
                ("step", (step + 1).to_string()),
                ("step concurrency", concurrency.to_string()),
            ]),
        )?;
        total_duration += duration;
        reports.push(report);
    }

    let merged = merge_reports(
        reports,
        total_duration,
        &args,
        &spec.url,
        spec.method.as_str(),
    )?;
    finish_report(&merged, &args.output)
}

fn ramp_concurrency(start: usize, end: usize, step: usize, steps: usize) -> usize {
    if steps <= 1 {
        return end;
    }
    let step = step as f64;
    let steps = (steps - 1) as f64;
    let start = start as f64;
    let end = end as f64;
    (start + (end - start) * (step / steps)).round() as usize
}

fn merge_reports(
    reports: Vec<TestReport>,
    total_duration: std::time::Duration,
    args: &RampArgs,
    url: &str,
    method: &str,
) -> Result<TestReport> {
    let mut iter = reports.into_iter();
    let Some(mut merged) = iter.next() else {
        anyhow::bail!("ramp produced no reports");
    };

    let mut completed = merged.completed;
    for report in iter {
        completed += report.completed;
        merged = merged.merge(report)?;
    }

    merged.command = "ramp".to_string();
    merged.url = url.to_string();
    merged.method = method.to_string();
    merged.total_duration = total_duration;
    merged.completed = completed;
    merged.metadata = metadata_map(&[
        ("requests", args.requests.to_string()),
        ("start concurrency", args.start_concurrency.to_string()),
        ("end concurrency", args.end_concurrency.to_string()),
        ("steps", args.steps.to_string()),
    ]);
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramp_concurrency_interpolates() {
        assert_eq!(ramp_concurrency(1, 10, 0, 5), 1);
        assert_eq!(ramp_concurrency(1, 10, 4, 5), 10);
        assert_eq!(ramp_concurrency(10, 20, 2, 5), 15);
    }

    #[test]
    fn ramp_concurrency_single_step() {
        assert_eq!(ramp_concurrency(5, 50, 0, 1), 50);
    }
}
