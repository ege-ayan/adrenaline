use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use anyhow::{Context, Result, bail};
use reqwest::Client;

use crate::request::{RequestSpec, send_request};
use crate::stats::{LocalStats, Stats};

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

/// Runs a load phase using a fixed worker pool. At most `concurrency` tasks exist;
/// workers pull work from a shared counter instead of spawning one task per request.
pub async fn run_phase(client: &Client, spec: &RequestSpec, phase: &LoadPhase) -> Result<Stats> {
    validate_phase(phase)?;

    let workers = phase.concurrency.min(phase.requests);
    let total = phase.requests;
    let next = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let client = client.clone();
        let spec = spec.clone();
        let next = Arc::clone(&next);

        handles.push(tokio::spawn(async move {
            let mut local = LocalStats::new()?;

            loop {
                let id = next.fetch_add(1, Ordering::Relaxed);
                if id >= total {
                    break;
                }

                let result = send_request(&client, &spec).await;
                local.record(result);
            }

            Ok::<LocalStats, anyhow::Error>(local)
        }));
    }

    let mut stats = Stats::new()?;
    for handle in handles {
        let local = handle.await.context("worker task panicked")??;
        stats.merge_local(local)?;
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

    #[tokio::test]
    async fn worker_pool_does_not_spawn_per_request_tasks() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .expect(200..=200)
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

        let (stats, _) = execute_load(&client, &spec, 200, 10).await.unwrap();
        let report = stats
            .finalize(
                "hit",
                &spec.url,
                "GET",
                Duration::from_secs(1),
                200,
                Default::default(),
            )
            .unwrap();

        assert_eq!(report.completed, 200);
        assert_eq!(report.successful, 200);
    }

    #[test]
    fn validate_phase_rejects_zero_requests() {
        assert!(
            validate_phase(&LoadPhase {
                requests: 0,
                concurrency: 1,
            })
            .is_err()
        );
    }
}
