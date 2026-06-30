use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{HitArgs, validate_hit_args};
use crate::output::request_spec_from_args;
use crate::report::{finish_report, metadata_map};
use crate::request::build_client;
use crate::runner::execute_load;

pub async fn run(args: HitArgs) -> Result<ExitCode> {
    validate_hit_args(&args)?;

    let spec = request_spec_from_args(&args.request).await?;
    let client = build_client(spec.timeout_secs).await?;

    let (stats, duration) = execute_load(&client, &spec, args.requests, args.concurrency).await?;

    let metadata = metadata_map(&[
        ("requests", args.requests.to_string()),
        ("concurrency", args.concurrency.to_string()),
        ("timeout", format!("{}s", args.request.timeout)),
    ]);

    let report = stats.finalize(
        "hit",
        &spec.url,
        spec.method.as_str(),
        duration,
        args.requests,
        metadata,
    )?;

    finish_report(&report, &args.output)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::cli::{OutputArgs, RequestArgs};
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn hit_against_mock_server() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let args = HitArgs {
            request: RequestArgs {
                url: server.uri(),
                method: HttpMethod::Get,
                headers: vec![],
                body: None,
                timeout: 10,
            },
            requests: 10,
            concurrency: 2,
            output: OutputArgs {
                json: false,
                html: None,
                baseline: None,
                save_baseline: None,
            },
        };

        let code = run(args).await.unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn hit_post_with_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let body = dir.path().join("body.json");
        std::fs::write(&body, r#"{"name":"test"}"#).unwrap();

        let args = HitArgs {
            request: RequestArgs {
                url: format!("{}/api", server.uri()),
                method: HttpMethod::Post,
                headers: vec!["Content-Type: application/json".to_string()],
                body: Some(body),
                timeout: 10,
            },
            requests: 5,
            concurrency: 2,
            output: OutputArgs {
                json: true,
                html: None,
                baseline: None,
                save_baseline: None,
            },
        };

        assert_eq!(run(args).await.unwrap(), ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn hit_writes_html_and_baseline() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(1)))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let html = dir.path().join("report.html");
        let baseline = dir.path().join("baseline.json");

        let args = HitArgs {
            request: RequestArgs {
                url: server.uri(),
                method: HttpMethod::Get,
                headers: vec![],
                body: None,
                timeout: 10,
            },
            requests: 3,
            concurrency: 1,
            output: OutputArgs {
                json: false,
                html: Some(html.clone()),
                baseline: None,
                save_baseline: Some(baseline.clone()),
            },
        };

        run(args).await.unwrap();
        assert!(html.exists());
        assert!(baseline.exists());
    }
}
