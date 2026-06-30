use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::RequestArgs;
use crate::request::{RequestSpec, parse_headers};
use crate::stats::{ReportSnapshot, TestReport};

pub async fn request_spec_from_args(args: &RequestArgs) -> Result<RequestSpec> {
    let headers = parse_headers(&args.headers)?;
    let body = match &args.body {
        Some(path) => Some(
            fs::read_to_string(path)
                .with_context(|| format!("failed to read body file: {}", path.display()))?,
        ),
        None => None,
    };

    let spec = RequestSpec {
        url: args.url.clone(),
        method: args.method,
        headers,
        body,
        timeout_secs: args.timeout,
    };
    spec.validate()?;
    Ok(spec)
}

pub fn render_report(report: &TestReport, json: bool) -> String {
    if json {
        serde_json::to_string_pretty(&report.snapshot()).expect("snapshot serializes")
    } else {
        render_text(report)
    }
}

pub fn render_text(report: &TestReport) -> String {
    let mut out = String::new();

    out.push_str(&format!("Adrenaline {}\n\n", report.command));
    out.push_str(&format!("Target:       {}\n", report.url));
    out.push_str(&format!("Method:       {}\n", report.method));

    if let Some(requests) = report.metadata.get("requests") {
        out.push_str(&format!("Requests:     {requests}\n"));
    }
    if let Some(concurrency) = report.metadata.get("concurrency") {
        out.push_str(&format!("Concurrency:  {concurrency}\n"));
    }
    for (key, value) in &report.metadata {
        if key != "requests" && key != "concurrency" {
            out.push_str(&format!("{key:12}  {value}\n"));
        }
    }
    out.push('\n');

    out.push_str("Summary\n");
    out.push_str("-------\n");
    out.push_str(&format!(
        "Total time:   {:.2}s\n",
        report.total_duration.as_secs_f64()
    ));
    out.push_str(&format!("Requests/sec: {:.2}\n", report.requests_per_sec()));
    out.push_str(&format!("Completed:    {}\n", report.completed));
    out.push_str(&format!("Successful:   {}\n", report.successful));
    out.push_str(&format!("Failed:       {}\n", report.failed));
    out.push_str(&format!("Error rate:   {:.2}%\n\n", report.error_rate()));

    let snap = report.snapshot();
    out.push_str("Latency\n");
    out.push_str("-------\n");
    out.push_str(&format!("min:  {}\n", format_ms(snap.latency.min_ms)));
    out.push_str(&format!("p50:  {}\n", format_ms(snap.latency.p50_ms)));
    out.push_str(&format!("p90:  {}\n", format_ms(snap.latency.p90_ms)));
    out.push_str(&format!("p95:  {}\n", format_ms(snap.latency.p95_ms)));
    out.push_str(&format!("p99:  {}\n", format_ms(snap.latency.p99_ms)));
    out.push_str(&format!("max:  {}\n\n", format_ms(snap.latency.max_ms)));

    out.push_str("Status codes\n");
    out.push_str("------------\n");
    if report.status_codes.is_empty() {
        out.push('\n');
        out.push_str("(none)\n");
    } else {
        for (code, count) in &report.status_codes {
            out.push_str(&format!("{code}: {count}\n"));
        }
    }

    if !report.errors.is_empty() {
        out.push('\n');
        out.push_str("Errors\n");
        out.push_str("------\n");
        for (error, count) in &report.errors {
            out.push_str(&format!("{error}: {count}\n"));
        }
    }

    out
}

pub fn render_html(report: &TestReport) -> String {
    let snap = report.snapshot();
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Adrenaline Report - {command}</title>
  <style>
    body {{ font-family: system-ui, sans-serif; margin: 2rem; color: #111; }}
    h1 {{ margin-bottom: 0.2rem; }}
    .meta {{ color: #555; margin-bottom: 2rem; }}
    table {{ border-collapse: collapse; margin-bottom: 1.5rem; min-width: 320px; }}
    th, td {{ border: 1px solid #ddd; padding: 0.5rem 0.75rem; text-align: left; }}
    th {{ background: #f5f5f5; }}
  </style>
</head>
<body>
  <h1>Adrenaline {command}</h1>
  <p class="meta">{method} {url}</p>
  <h2>Summary</h2>
  <table>
    <tr><th>Total time</th><td>{total_time:.2}s</td></tr>
    <tr><th>Requests/sec</th><td>{rps:.2}</td></tr>
    <tr><th>Completed</th><td>{completed}</td></tr>
    <tr><th>Successful</th><td>{successful}</td></tr>
    <tr><th>Failed</th><td>{failed}</td></tr>
    <tr><th>Error rate</th><td>{error_rate:.2}%</td></tr>
  </table>
  <h2>Latency</h2>
  <table>
    <tr><th>min</th><td>{min}ms</td></tr>
    <tr><th>p50</th><td>{p50}ms</td></tr>
    <tr><th>p90</th><td>{p90}ms</td></tr>
    <tr><th>p95</th><td>{p95}ms</td></tr>
    <tr><th>p99</th><td>{p99}ms</td></tr>
    <tr><th>max</th><td>{max}ms</td></tr>
  </table>
  <h2>Status codes</h2>
  <table>{status_rows}</table>
  <h2>Errors</h2>
  <table>{error_rows}</table>
</body>
</html>"#,
        command = report.command,
        method = report.method,
        url = report.url,
        total_time = snap.total_duration_secs,
        rps = snap.requests_per_sec,
        completed = snap.completed,
        successful = snap.successful,
        failed = snap.failed,
        error_rate = snap.error_rate,
        min = snap.latency.min_ms,
        p50 = snap.latency.p50_ms,
        p90 = snap.latency.p90_ms,
        p95 = snap.latency.p95_ms,
        p99 = snap.latency.p99_ms,
        max = snap.latency.max_ms,
        status_rows = table_rows(&snap.status_codes),
        error_rows = table_rows_str(&snap.errors),
    )
}

fn table_rows(codes: &BTreeMap<u16, usize>) -> String {
    if codes.is_empty() {
        return "<tr><td colspan=\"2\">(none)</td></tr>".to_string();
    }
    codes
        .iter()
        .map(|(code, count)| format!("<tr><td>{code}</td><td>{count}</td></tr>"))
        .collect()
}

fn table_rows_str(items: &BTreeMap<String, usize>) -> String {
    if items.is_empty() {
        return "<tr><td colspan=\"2\">(none)</td></tr>".to_string();
    }
    items
        .iter()
        .map(|(key, count)| format!("<tr><td>{key}</td><td>{count}</td></tr>"))
        .collect()
}

pub fn format_ms(ms: f64) -> String {
    if ms >= 100.0 {
        format!("{ms:.0}")
    } else if ms >= 10.0 {
        format!("{ms:.1}")
    } else {
        format!("{ms:.2}")
    }
}

pub fn write_html_report(report: &TestReport, path: &Path) -> Result<()> {
    fs::write(path, render_html(report))
        .with_context(|| format!("failed to write HTML report: {}", path.display()))
}

pub fn write_baseline(report: &TestReport, path: &Path) -> Result<()> {
    let json =
        serde_json::to_string_pretty(&report.snapshot()).context("failed to serialize baseline")?;
    fs::write(path, json).with_context(|| format!("failed to write baseline: {}", path.display()))
}

pub fn load_baseline(path: &Path) -> Result<ReportSnapshot> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read baseline: {}", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("invalid baseline JSON: {}", path.display()))
}

pub fn print_comparison(lines: &[String]) {
    println!("Baseline comparison\n");
    for line in lines {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use hdrhistogram::Histogram;

    use super::*;
    use crate::request::HttpMethod;

    fn sample_report() -> TestReport {
        let mut histogram = Histogram::<u64>::new(3).unwrap();
        histogram.record(12_000).unwrap();
        histogram.record(34_000).unwrap();
        histogram.record(590_000).unwrap();

        let mut metadata = BTreeMap::new();
        metadata.insert("requests".to_string(), "1000".to_string());
        metadata.insert("concurrency".to_string(), "50".to_string());

        TestReport {
            command: "hit".to_string(),
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            total_duration: Duration::from_secs_f64(2.41),
            completed: 1000,
            successful: 998,
            failed: 2,
            histogram,
            status_codes: BTreeMap::from([(200, 998), (500, 2)]),
            errors: BTreeMap::from([
                ("timeout".to_string(), 1),
                ("connection error".to_string(), 1),
            ]),
            metadata,
        }
    }

    #[test]
    fn render_text_contains_sections() {
        let text = render_text(&sample_report());
        assert!(text.contains("Adrenaline hit"));
        assert!(text.contains("Summary"));
        assert!(text.contains("Latency"));
        assert!(text.contains("Status codes"));
        assert!(text.contains("200: 998"));
    }

    #[test]
    fn render_json_is_valid() {
        let json = render_report(&sample_report(), true);
        let parsed: ReportSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.completed, 1000);
        assert_eq!(parsed.method, "GET");
    }

    #[test]
    fn render_html_contains_tables() {
        let html = render_html(&sample_report());
        assert!(html.contains("<html"));
        assert!(html.contains("Adrenaline hit"));
        assert!(html.contains("p99"));
    }

    #[test]
    fn format_ms_precision() {
        assert_eq!(format_ms(102.4), "102");
        assert_eq!(format_ms(68.9), "68.9");
        assert_eq!(format_ms(5.12), "5.12");
    }

    #[tokio::test]
    async fn request_spec_reads_body_file() {
        let dir = tempfile::tempdir().unwrap();
        let body_path = dir.path().join("body.json");
        fs::write(&body_path, r#"{"ok":true}"#).unwrap();

        let args = RequestArgs {
            url: "https://example.com".to_string(),
            method: HttpMethod::Post,
            headers: vec!["Content-Type: application/json".to_string()],
            body: Some(body_path),
            timeout: 10,
        };

        let spec = request_spec_from_args(&args).await.unwrap();
        assert_eq!(spec.body.as_deref(), Some(r#"{"ok":true}"#));
    }
}
