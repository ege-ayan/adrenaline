use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use reqwest::Client;
use tokio::sync::Semaphore;

use crate::request::{RequestSpec, send_request};
use crate::stats::Stats;

#[derive(Debug, Clone)]
pub struct LoadPhase {
    pub requests: usize,
    pub concurrency: usize,
}

pub fn validate_phase(phase: &LoadPhase) -> Result<()> {
    if phase.requests == 0 {
        bail!("requests must be greater than 0");
    }
    if phase.concurrency == 0 {
        bail!("concurrency must be greater than 0");
    }
    Ok(())
}

pub async fn run_phase(client: &Client, spec: &RequestSpec, phase: &LoadPhase) -> Result<Stats> {
    validate_phase(phase)?;

    let semaphore = Arc::new(Semaphore::new(phase.concurrency));
    let stats = Stats::new()?;

    let mut handles = Vec::with_capacity(phase.requests);
    for _ in 0..phase.requests {
        let client = client.clone();
        let spec = spec.clone();
        let semaphore = Arc::clone(&semaphore);
        let stats = stats.clone_handles();

        handles.push(tokio::spawn(async move {
            let _permit = semaphore.acquire().await.expect("semaphore closed");
            let result = send_request(&client, &spec).await;
            stats.record(result).await;
        }));
    }

    for handle in handles {
        handle.await.context("request task panicked")?;
    }

    Ok(stats)
}

pub async fn execute_load(
    client: &Client,
    spec: &RequestSpec,
    requests: usize,
    concurrency: usize,
) -> Result<(Stats, std::time::Duration)> {
    let phase = LoadPhase {
        requests,
        concurrency,
    };
    let start = Instant::now();
    let stats = run_phase(client, spec, &phase).await?;
    Ok((stats, start.elapsed()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::request::HttpMethod;

    #[tokio::test]
    async fn execute_load_respects_concurrency() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(50)))
            .expect(20..=20)
            .mount(&server)
            .await;

        let spec = RequestSpec {
            url: server.uri(),
            method: HttpMethod::Get,
            headers: vec![],
            body: None,
            timeout_secs: 10,
        };
        let client = crate::request::build_client(10).await.unwrap();

        let (stats, duration) = execute_load(&client, &spec, 20, 5).await.unwrap();
        let report = stats
            .finalize("hit", &spec.url, "GET", duration, 20, Default::default())
            .unwrap();

        assert_eq!(report.completed, 20);
        assert_eq!(report.successful, 20);
    }

    #[test]
    fn validate_phase_rejects_zero_requests() {
        assert!(validate_phase(&LoadPhase {
            requests: 0,
            concurrency: 1,
        })
        .is_err());
    }
}
