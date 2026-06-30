use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestResult {
    pub latency: Duration,
    pub status: Option<u16>,
    pub error: Option<String>,
}

pub fn is_successful_status(status: u16) -> bool {
    (200..400).contains(&status)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LatencySnapshot {
    pub min_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

impl LatencySnapshot {
    pub fn from_histogram(histogram: &Histogram<u64>) -> Self {
        Self {
            min_ms: micros_to_ms(histogram.min()),
            p50_ms: micros_to_ms(histogram.value_at_quantile(0.50)),
            p90_ms: micros_to_ms(histogram.value_at_quantile(0.90)),
            p95_ms: micros_to_ms(histogram.value_at_quantile(0.95)),
            p99_ms: micros_to_ms(histogram.value_at_quantile(0.99)),
            max_ms: micros_to_ms(histogram.max()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportSnapshot {
    pub command: String,
    pub url: String,
    pub method: String,
    pub total_duration_secs: f64,
    pub completed: usize,
    pub successful: usize,
    pub failed: usize,
    pub error_rate: f64,
    pub requests_per_sec: f64,
    pub latency: LatencySnapshot,
    pub status_codes: BTreeMap<u16, usize>,
    pub errors: BTreeMap<String, usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BTreeMap<String, String>>,
}

pub struct TestReport {
    pub command: String,
    pub url: String,
    pub method: String,
    pub total_duration: Duration,
    pub completed: usize,
    pub successful: usize,
    pub failed: usize,
    pub histogram: Histogram<u64>,
    pub status_codes: BTreeMap<u16, usize>,
    pub errors: BTreeMap<String, usize>,
    pub metadata: BTreeMap<String, String>,
}

impl TestReport {
    pub fn error_rate(&self) -> f64 {
        if self.completed == 0 {
            0.0
        } else {
            (self.failed as f64 / self.completed as f64) * 100.0
        }
    }

    pub fn requests_per_sec(&self) -> f64 {
        let secs = self.total_duration.as_secs_f64();
        if secs > 0.0 {
            self.completed as f64 / secs
        } else {
            0.0
        }
    }

    pub fn snapshot(&self) -> ReportSnapshot {
        ReportSnapshot {
            command: self.command.clone(),
            url: self.url.clone(),
            method: self.method.clone(),
            total_duration_secs: self.total_duration.as_secs_f64(),
            completed: self.completed,
            successful: self.successful,
            failed: self.failed,
            error_rate: self.error_rate(),
            requests_per_sec: self.requests_per_sec(),
            latency: LatencySnapshot::from_histogram(&self.histogram),
            status_codes: self.status_codes.clone(),
            errors: self.errors.clone(),
            metadata: if self.metadata.is_empty() {
                None
            } else {
                Some(self.metadata.clone())
            },
        }
    }

    pub fn merge(mut self, other: TestReport) -> Result<Self> {
        self.total_duration += other.total_duration;
        self.completed += other.completed;
        self.successful += other.successful;
        self.failed += other.failed;
        self.histogram
            .add(other.histogram)
            .context("failed to merge histograms")?;

        for (code, count) in other.status_codes {
            *self.status_codes.entry(code).or_insert(0) += count;
        }
        for (error, count) in other.errors {
            *self.errors.entry(error).or_insert(0) += count;
        }
        self.metadata.extend(other.metadata);
        Ok(self)
    }
}

pub struct Stats {
    histogram: Arc<Mutex<Histogram<u64>>>,
    status_codes: Arc<Mutex<BTreeMap<u16, usize>>>,
    errors: Arc<Mutex<BTreeMap<String, usize>>>,
    successful: Arc<Mutex<usize>>,
    failed: Arc<Mutex<usize>>,
}

impl Stats {
    pub fn new() -> Result<Self> {
        Ok(Self {
            histogram: Arc::new(Mutex::new(
                Histogram::<u64>::new(3).context("failed to create latency histogram")?,
            )),
            status_codes: Arc::new(Mutex::new(BTreeMap::new())),
            errors: Arc::new(Mutex::new(BTreeMap::new())),
            successful: Arc::new(Mutex::new(0)),
            failed: Arc::new(Mutex::new(0)),
        })
    }

    pub fn clone_handles(&self) -> Self {
        Self {
            histogram: Arc::clone(&self.histogram),
            status_codes: Arc::clone(&self.status_codes),
            errors: Arc::clone(&self.errors),
            successful: Arc::clone(&self.successful),
            failed: Arc::clone(&self.failed),
        }
    }

    pub async fn record(&self, result: RequestResult) {
        let micros = result.latency.as_micros() as u64;
        {
            let mut hist = self.histogram.lock().await;
            hist.record(micros).expect("histogram record failed");
        }

        if let Some(status) = result.status {
            *self.status_codes.lock().await.entry(status).or_insert(0) += 1;

            if is_successful_status(status) {
                *self.successful.lock().await += 1;
            } else {
                *self.failed.lock().await += 1;
            }
        } else {
            *self.failed.lock().await += 1;

            if let Some(error) = result.error {
                *self.errors.lock().await.entry(error).or_insert(0) += 1;
            }
        }
    }

    pub fn finalize(
        self,
        command: &str,
        url: &str,
        method: &str,
        total_duration: Duration,
        completed: usize,
        metadata: BTreeMap<String, String>,
    ) -> Result<TestReport> {
        let histogram = Arc::try_unwrap(self.histogram)
            .map_err(|_| anyhow::anyhow!("histogram still in use"))?
            .into_inner();
        let status_codes = Arc::try_unwrap(self.status_codes)
            .map_err(|_| anyhow::anyhow!("status codes still in use"))?
            .into_inner();
        let errors = Arc::try_unwrap(self.errors)
            .map_err(|_| anyhow::anyhow!("errors still in use"))?
            .into_inner();
        let successful = Arc::try_unwrap(self.successful)
            .map_err(|_| anyhow::anyhow!("successful counter still in use"))?
            .into_inner();
        let failed = Arc::try_unwrap(self.failed)
            .map_err(|_| anyhow::anyhow!("failed counter still in use"))?
            .into_inner();

        Ok(TestReport {
            command: command.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            total_duration,
            completed,
            successful,
            failed,
            histogram,
            status_codes,
            errors,
            metadata,
        })
    }
}

pub fn micros_to_ms(value: u64) -> f64 {
    value as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_successful_status_boundaries() {
        assert!(is_successful_status(200));
        assert!(is_successful_status(399));
        assert!(!is_successful_status(400));
        assert!(!is_successful_status(500));
    }

    #[tokio::test]
    async fn stats_records_success_and_failure() {
        let stats = Stats::new().unwrap();

        stats
            .record(RequestResult {
                latency: Duration::from_millis(10),
                status: Some(200),
                error: None,
            })
            .await;
        stats
            .record(RequestResult {
                latency: Duration::from_millis(20),
                status: Some(500),
                error: None,
            })
            .await;
        stats
            .record(RequestResult {
                latency: Duration::from_millis(30),
                status: None,
                error: Some("timeout".to_string()),
            })
            .await;

        let report = stats
            .finalize(
                "hit",
                "https://example.com",
                "GET",
                Duration::from_secs(1),
                3,
                BTreeMap::new(),
            )
            .unwrap();

        assert_eq!(report.completed, 3);
        assert_eq!(report.successful, 1);
        assert_eq!(report.failed, 2);
        assert_eq!(report.status_codes.get(&200), Some(&1));
        assert_eq!(report.status_codes.get(&500), Some(&1));
        assert_eq!(report.errors.get("timeout"), Some(&1));
    }

    #[tokio::test]
    async fn merge_combines_reports() {
        let stats_a = Stats::new().unwrap();
        stats_a
            .record(RequestResult {
                latency: Duration::from_millis(10),
                status: Some(200),
                error: None,
            })
            .await;
        let report_a = stats_a
            .finalize("hit", "https://a.com", "GET", Duration::from_secs(1), 1, BTreeMap::new())
            .unwrap();

        let stats_b = Stats::new().unwrap();
        stats_b
            .record(RequestResult {
                latency: Duration::from_millis(20),
                status: Some(200),
                error: None,
            })
            .await;
        let report_b = stats_b
            .finalize("hit", "https://a.com", "GET", Duration::from_secs(1), 1, BTreeMap::new())
            .unwrap();

        let merged = report_a.merge(report_b).unwrap();
        assert_eq!(merged.completed, 2);
        assert_eq!(merged.successful, 2);
    }

    #[test]
    fn snapshot_contains_error_rate() {
        let mut histogram = Histogram::<u64>::new(3).unwrap();
        histogram.record(10_000).unwrap();

        let report = TestReport {
            command: "hit".to_string(),
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            total_duration: Duration::from_secs(2),
            completed: 10,
            successful: 8,
            failed: 2,
            histogram,
            status_codes: BTreeMap::new(),
            errors: BTreeMap::new(),
            metadata: BTreeMap::new(),
        };

        let snap = report.snapshot();
        assert_eq!(snap.error_rate, 20.0);
        assert_eq!(snap.requests_per_sec, 5.0);
    }
}
