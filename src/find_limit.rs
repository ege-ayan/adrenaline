use std::collections::BTreeMap;
use std::process::ExitCode;

use anyhow::{Context, Result};

use crate::cli::{FindLimitArgs, validate_find_limit_args};
use crate::output::request_spec_from_args;
use crate::report::finish_report;
use crate::request::build_client;
use crate::runner::execute_load;

use crate::stats::TestReport;

pub async fn run(args: FindLimitArgs) -> Result<ExitCode> {
    validate_find_limit_args(&args)?;

    let spec = request_spec_from_args(&args.request).await?;
    let client = build_client(spec.timeout_secs).await?;

    let mut last_good = 0usize;
    let mut last_report: Option<TestReport> = None;
    let mut total_duration = std::time::Duration::ZERO;

    let mut concurrency = args.start_concurrency;
    while concurrency <= args.max_concurrency {
        let (stats, duration) = execute_load(
            &client,
            &spec,
            args.requests_per_step,
            concurrency,
        )
        .await?;

        let report = stats.finalize(
            "find-limit",
            &spec.url,
            spec.method.as_str(),
            duration,
            args.requests_per_step,
            BTreeMap::from([(
                "concurrency tested".to_string(),
                concurrency.to_string(),
            )]),
        )?;

        total_duration += duration;

        if report.error_rate() <= args.max_error_rate {
            last_good = concurrency;
            last_report = Some(report);
        } else if last_report.is_none() {
            last_report = Some(report);
            break;
        } else {
            break;
        }

        if concurrency == args.max_concurrency {
            break;
        }

        concurrency = (concurrency + args.step).min(args.max_concurrency);
    }

    let mut final_report = last_report.context("find-limit produced no results")?;

    final_report.command = "find-limit".to_string();
    final_report.total_duration = total_duration;
    final_report.metadata.insert("limit found".to_string(), last_good.to_string());
    final_report.metadata.insert(
        "requests per step".to_string(),
        args.requests_per_step.to_string(),
    );
    final_report.metadata.insert(
        "max error rate".to_string(),
        format!("{}%", args.max_error_rate),
    );
    final_report.metadata.insert("max concurrency".to_string(), args.max_concurrency.to_string());

    finish_report(&final_report, &args.output)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::cli::{OutputArgs, RequestArgs};
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn find_limit_stops_when_error_rate_exceeded() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .up_to_n_times(50)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let args = FindLimitArgs {
            request: RequestArgs {
                url: server.uri(),
                method: HttpMethod::Get,
                headers: vec![],
                body: None,
                timeout: 10,
            },
            requests_per_step: 20,
            start_concurrency: 5,
            max_concurrency: 50,
            step: 10,
            max_error_rate: 0.0,
            output: OutputArgs {
                json: false,
                html: None,
                baseline: None,
                save_baseline: None,
            },
        };

        assert_eq!(run(args).await.unwrap(), ExitCode::SUCCESS);
    }
}
