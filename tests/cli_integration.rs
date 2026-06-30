use assert_cmd::Command;
use predicates::prelude::*;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn cli_help_shows_all_commands() {
    Command::cargo_bin("adrenaline")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("hit"))
        .stdout(predicate::str::contains("ramp"))
        .stdout(predicate::str::contains("spike"))
        .stdout(predicate::str::contains("find-limit"))
        .stdout(predicate::str::contains("compare"))
        .stdout(predicate::str::contains("scenario"));
}

#[test]
fn cli_hit_rejects_zero_requests() {
    Command::cargo_bin("adrenaline")
        .unwrap()
        .args(["hit", "https://example.com", "-n", "0", "-c", "10"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requests must be greater than 0"));
}

#[test]
fn cli_hit_rejects_zero_concurrency() {
    Command::cargo_bin("adrenaline")
        .unwrap()
        .args(["hit", "https://example.com", "-n", "10", "-c", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "concurrency must be greater than 0",
        ));
}

#[test]
fn cli_hit_rejects_invalid_url() {
    Command::cargo_bin("adrenaline")
        .unwrap()
        .args(["hit", "not-a-url", "-n", "1", "-c", "1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid URL"));
}

#[test]
fn cli_hit_json_output() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        Command::cargo_bin("adrenaline")
            .unwrap()
            .args(["hit", &server.uri(), "-n", "5", "-c", "2", "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains("\"completed\": 5"))
            .stdout(predicate::str::contains("\"method\": \"GET\""));
    });
}

#[test]
fn cli_compare_detects_regression() {
    let dir = tempfile::tempdir().unwrap();
    let baseline = dir.path().join("baseline.json");
    let current = dir.path().join("current.json");

    std::fs::write(
        &baseline,
        r#"{
            "command": "hit",
            "url": "https://example.com",
            "method": "GET",
            "total_duration_secs": 1.0,
            "completed": 100,
            "successful": 100,
            "failed": 0,
            "error_rate": 0.0,
            "requests_per_sec": 100.0,
            "latency": {"min_ms": 1.0, "p50_ms": 10.0, "p90_ms": 20.0, "p95_ms": 25.0, "p99_ms": 100.0, "max_ms": 100.0},
            "status_codes": {"200": 100},
            "errors": {}
        }"#,
    )
    .unwrap();

    std::fs::write(
        &current,
        r#"{
            "command": "hit",
            "url": "https://example.com",
            "method": "GET",
            "total_duration_secs": 1.0,
            "completed": 100,
            "successful": 90,
            "failed": 10,
            "error_rate": 10.0,
            "requests_per_sec": 100.0,
            "latency": {"min_ms": 1.0, "p50_ms": 10.0, "p90_ms": 20.0, "p95_ms": 25.0, "p99_ms": 100.0, "max_ms": 100.0},
            "status_codes": {"200": 90, "500": 10},
            "errors": {}
        }"#,
    )
    .unwrap();

    Command::cargo_bin("adrenaline")
        .unwrap()
        .args([
            "compare",
            baseline.to_str().unwrap(),
            current.to_str().unwrap(),
        ])
        .assert()
        .code(1);
}

#[test]
fn cli_scenario_runs_yaml_file() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let scenario = dir.path().join("scenario.yaml");
        std::fs::write(
            &scenario,
            format!(
                "name: integration\nsteps:\n  - type: hit\n    url: {}\n    requests: 2\n    concurrency: 1\n",
                server.uri()
            ),
        )
        .unwrap();

        Command::cargo_bin("adrenaline")
            .unwrap()
            .args(["scenario", scenario.to_str().unwrap(), "--json"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Running scenario: integration"));
    });
}

#[test]
fn cli_hit_writes_html_report() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let html = dir.path().join("report.html");

        Command::cargo_bin("adrenaline")
            .unwrap()
            .args([
                "hit",
                &server.uri(),
                "-n",
                "2",
                "-c",
                "1",
                "--html",
                html.to_str().unwrap(),
            ])
            .assert()
            .success();

        assert!(html.exists());
        let content = std::fs::read_to_string(html).unwrap();
        assert!(content.contains("<html"));
    });
}
