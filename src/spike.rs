use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{SpikeArgs, validate_spike_args};
use crate::output::request_spec_from_args;
use crate::report::finish_report;
use crate::request::build_client;
use crate::runner::execute_load;

pub async fn run(args: SpikeArgs) -> Result<ExitCode> {
    validate_spike_args(&args)?;

    let spec = request_spec_from_args(&args.request).await?;
    let client = build_client(spec.timeout_secs).await?;

    let (baseline_stats, baseline_duration) = execute_load(
        &client,
        &spec,
        args.baseline_requests,
        args.baseline_concurrency,
    )
    .await?;

    let baseline_report = baseline_stats.finalize(
        "spike",
        &spec.url,
        spec.method.as_str(),
        baseline_duration,
        args.baseline_requests,
        [
            ("phase".to_string(), "baseline".to_string()),
            (
                "baseline requests".to_string(),
                args.baseline_requests.to_string(),
            ),
            (
                "baseline concurrency".to_string(),
                args.baseline_concurrency.to_string(),
            ),
            (
                "spike requests".to_string(),
                args.spike_requests.to_string(),
            ),
            (
                "spike concurrency".to_string(),
                args.spike_concurrency.to_string(),
            ),
        ]
        .into_iter()
        .collect(),
    )?;

    let (spike_stats, spike_duration) =
        execute_load(&client, &spec, args.spike_requests, args.spike_concurrency).await?;

    let spike_report = spike_stats.finalize(
        "spike",
        &spec.url,
        spec.method.as_str(),
        spike_duration,
        args.spike_requests,
        Default::default(),
    )?;

    let total_duration = baseline_duration + spike_duration;
    let mut merged = baseline_report.merge(spike_report)?;
    merged.command = "spike".to_string();
    merged.total_duration = total_duration;
    merged.completed = args.baseline_requests + args.spike_requests;
    merged
        .metadata
        .insert("phase".to_string(), "combined".to_string());
    merged.metadata.insert(
        "baseline requests".to_string(),
        args.baseline_requests.to_string(),
    );
    merged.metadata.insert(
        "baseline concurrency".to_string(),
        args.baseline_concurrency.to_string(),
    );
    merged.metadata.insert(
        "spike requests".to_string(),
        args.spike_requests.to_string(),
    );
    merged.metadata.insert(
        "spike concurrency".to_string(),
        args.spike_concurrency.to_string(),
    );

    finish_report(&merged, &args.output)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::cli::{OutputArgs, RequestArgs};
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn spike_runs_baseline_and_spike_phases() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let args = SpikeArgs {
            request: RequestArgs {
                url: server.uri(),
                method: HttpMethod::Get,
                headers: vec![],
                body: None,
                timeout: 10,
            },
            baseline_requests: 5,
            baseline_concurrency: 1,
            spike_requests: 10,
            spike_concurrency: 3,
            output: OutputArgs {
                json: true,
                html: None,
                baseline: None,
                save_baseline: None,
            },
        };

        assert_eq!(run(args).await.unwrap(), ExitCode::SUCCESS);
    }
}
