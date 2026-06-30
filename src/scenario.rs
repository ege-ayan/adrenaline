use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::cli::{HitArgs, OutputArgs, RampArgs, RequestArgs, ScenarioArgs, SpikeArgs};
use crate::find_limit;
use crate::hit;
use crate::ramp;
use crate::request::HttpMethod;
use crate::spike;

#[derive(Debug, Deserialize)]
pub struct ScenarioFile {
    pub name: String,
    #[serde(default)]
    pub defaults: ScenarioDefaults,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Default, Deserialize)]
pub struct ScenarioDefaults {
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default)]
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: Vec<String>,
}

fn default_timeout() -> u64 {
    10
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ScenarioStep {
    Hit(ScenarioHit),
    Ramp(ScenarioRamp),
    Spike(ScenarioSpike),
    FindLimit(ScenarioFindLimit),
}

#[derive(Debug, Deserialize)]
pub struct ScenarioHit {
    pub url: String,
    #[serde(default = "default_requests")]
    pub requests: usize,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    #[serde(default)]
    pub headers: Vec<String>,
    #[serde(default)]
    pub body: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioRamp {
    pub url: String,
    #[serde(default = "default_requests")]
    pub requests: usize,
    #[serde(default = "default_start_concurrency")]
    pub start_concurrency: usize,
    #[serde(default = "default_concurrency")]
    pub end_concurrency: usize,
    #[serde(default = "default_steps")]
    pub steps: usize,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    #[serde(default)]
    pub headers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioSpike {
    pub url: String,
    #[serde(default = "default_baseline_requests")]
    pub baseline_requests: usize,
    #[serde(default = "default_start_concurrency")]
    pub baseline_concurrency: usize,
    #[serde(default = "default_spike_requests")]
    pub spike_requests: usize,
    #[serde(default = "default_spike_concurrency")]
    pub spike_concurrency: usize,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    #[serde(default)]
    pub headers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScenarioFindLimit {
    pub url: String,
    #[serde(default = "default_requests_per_step")]
    pub requests_per_step: usize,
    #[serde(default = "default_start_concurrency")]
    pub start_concurrency: usize,
    #[serde(default = "default_max_concurrency")]
    pub max_concurrency: usize,
    #[serde(default = "default_step")]
    pub step: usize,
    #[serde(default = "default_max_error_rate")]
    pub max_error_rate: f64,
    #[serde(default)]
    pub method: Option<HttpMethod>,
    #[serde(default)]
    pub headers: Vec<String>,
}

fn default_requests() -> usize {
    100
}
fn default_concurrency() -> usize {
    10
}
fn default_start_concurrency() -> usize {
    1
}
fn default_steps() -> usize {
    5
}
fn default_baseline_requests() -> usize {
    50
}
fn default_spike_requests() -> usize {
    200
}
fn default_spike_concurrency() -> usize {
    50
}
fn default_requests_per_step() -> usize {
    50
}
fn default_max_concurrency() -> usize {
    200
}
fn default_step() -> usize {
    10
}
fn default_max_error_rate() -> f64 {
    5.0
}

pub fn parse_scenario(content: &str) -> Result<ScenarioFile> {
    serde_yaml::from_str(content).context("failed to parse scenario YAML")
}

pub fn load_scenario(path: &Path) -> Result<ScenarioFile> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read scenario: {}", path.display()))?;
    parse_scenario(&content)
}

pub async fn run(args: ScenarioArgs) -> Result<ExitCode> {
    let scenario = load_scenario(&args.file)?;
    if scenario.steps.is_empty() {
        bail!("scenario must contain at least one step");
    }

    println!("Running scenario: {}\n", scenario.name);

    for (index, step) in scenario.steps.iter().enumerate() {
        println!("Step {}/{}: {}", index + 1, scenario.steps.len(), step_type(step));

        let output = OutputArgs {
            json: args.output.json,
            html: None,
            baseline: None,
            save_baseline: None,
        };

        match step {
            ScenarioStep::Hit(step_args) => {
                let hit_args = HitArgs {
                    request: build_request_args(
                        step_args.url.clone(),
                        &scenario.defaults,
                        step_args.method,
                        &step_args.headers,
                        None,
                    ),
                    requests: step_args.requests,
                    concurrency: step_args.concurrency,
                    output: output.clone(),
                };
                hit::run(hit_args).await?;
            }
            ScenarioStep::Ramp(step_args) => {
                let ramp_args = RampArgs {
                    request: build_request_args(
                        step_args.url.clone(),
                        &scenario.defaults,
                        step_args.method,
                        &step_args.headers,
                        None,
                    ),
                    requests: step_args.requests,
                    start_concurrency: step_args.start_concurrency,
                    end_concurrency: step_args.end_concurrency,
                    steps: step_args.steps,
                    output: output.clone(),
                };
                ramp::run(ramp_args).await?;
            }
            ScenarioStep::Spike(step_args) => {
                let spike_args = SpikeArgs {
                    request: build_request_args(
                        step_args.url.clone(),
                        &scenario.defaults,
                        step_args.method,
                        &step_args.headers,
                        None,
                    ),
                    baseline_requests: step_args.baseline_requests,
                    baseline_concurrency: step_args.baseline_concurrency,
                    spike_requests: step_args.spike_requests,
                    spike_concurrency: step_args.spike_concurrency,
                    output: output.clone(),
                };
                spike::run(spike_args).await?;
            }
            ScenarioStep::FindLimit(step_args) => {
                let find_args = crate::cli::FindLimitArgs {
                    request: build_request_args(
                        step_args.url.clone(),
                        &scenario.defaults,
                        step_args.method,
                        &step_args.headers,
                        None,
                    ),
                    requests_per_step: step_args.requests_per_step,
                    start_concurrency: step_args.start_concurrency,
                    max_concurrency: step_args.max_concurrency,
                    step: step_args.step,
                    max_error_rate: step_args.max_error_rate,
                    output: output.clone(),
                };
                find_limit::run(find_args).await?;
            }
        }

        println!();
    }

    Ok(ExitCode::SUCCESS)
}

fn step_type(step: &ScenarioStep) -> &'static str {
    match step {
        ScenarioStep::Hit(_) => "hit",
        ScenarioStep::Ramp(_) => "ramp",
        ScenarioStep::Spike(_) => "spike",
        ScenarioStep::FindLimit(_) => "find-limit",
    }
}

fn build_request_args(
    url: String,
    defaults: &ScenarioDefaults,
    method: Option<HttpMethod>,
    headers: &[String],
    body_path: Option<std::path::PathBuf>,
) -> RequestArgs {
    let mut merged_headers = defaults.headers.clone();
    merged_headers.extend(headers.iter().cloned());

    RequestArgs {
        url,
        method: method.unwrap_or(defaults.method),
        headers: merged_headers,
        body: body_path,
        timeout: defaults.timeout,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scenario_yaml() {
        let yaml = r#"
name: smoke test
defaults:
  timeout: 5
  method: GET
steps:
  - type: hit
    url: https://example.com
    requests: 10
    concurrency: 2
  - type: ramp
    url: https://example.com
    requests: 100
    start_concurrency: 1
    end_concurrency: 10
    steps: 5
"#;
        let scenario = parse_scenario(yaml).unwrap();
        assert_eq!(scenario.name, "smoke test");
        assert_eq!(scenario.defaults.timeout, 5);
        assert_eq!(scenario.steps.len(), 2);
    }

    #[test]
    fn parse_scenario_rejects_invalid_yaml() {
        assert!(parse_scenario("not: [valid").is_err());
    }

    #[tokio::test]
    async fn run_scenario_file_against_mock_server() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("scenario.yaml");
        std::fs::write(
            &file,
            format!(
                r#"
name: local test
steps:
  - type: hit
    url: {}
    requests: 3
    concurrency: 1
"#,
                server.uri()
            ),
        )
        .unwrap();

        let args = ScenarioArgs {
            file,
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
