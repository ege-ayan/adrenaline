use crate::stats::ReportSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonLine {
    pub kind: ComparisonKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonKind {
    Regression,
    Improvement,
    Info,
}

pub struct ComparisonOptions {
    pub p99_threshold_pct: f64,
    pub error_rate_threshold_pts: f64,
}

pub fn compare_reports(
    baseline: &ReportSnapshot,
    current: &ReportSnapshot,
    options: &ComparisonOptions,
) -> Vec<ComparisonLine> {
    let mut lines = Vec::new();

    if baseline.url != current.url {
        lines.push(ComparisonLine {
            kind: ComparisonKind::Info,
            message: format!("URL changed: {} -> {}", baseline.url, current.url),
        });
    }

    compare_points(
        &mut lines,
        "error rate",
        baseline.error_rate,
        current.error_rate,
        options.error_rate_threshold_pts,
        true,
        "%",
    );

    compare_percent(
        &mut lines,
        "p99 latency",
        baseline.latency.p99_ms,
        current.latency.p99_ms,
        options.p99_threshold_pct,
        true,
        "ms",
    );

    compare_percent(
        &mut lines,
        "requests/sec",
        baseline.requests_per_sec,
        current.requests_per_sec,
        10.0,
        false,
        "rps",
    );

    compare_percent(
        &mut lines,
        "p50 latency",
        baseline.latency.p50_ms,
        current.latency.p50_ms,
        10.0,
        true,
        "ms",
    );

    if lines.is_empty() {
        lines.push(ComparisonLine {
            kind: ComparisonKind::Info,
            message: "No significant differences detected".to_string(),
        });
    }

    lines
}

fn compare_points(
    lines: &mut Vec<ComparisonLine>,
    label: &str,
    baseline: f64,
    current: f64,
    threshold_pts: f64,
    higher_is_bad: bool,
    unit: &str,
) {
    let delta = current - baseline;
    if delta.abs() <= threshold_pts {
        return;
    }

    let improvement = if higher_is_bad {
        delta < 0.0
    } else {
        delta > 0.0
    };
    push_delta_line(
        lines,
        label,
        baseline,
        current,
        delta,
        improvement,
        unit,
        "pts",
    );
}

fn compare_percent(
    lines: &mut Vec<ComparisonLine>,
    label: &str,
    baseline: f64,
    current: f64,
    threshold_pct: f64,
    higher_is_bad: bool,
    unit: &str,
) {
    if baseline == 0.0 {
        return;
    }

    let pct_change = ((current - baseline) / baseline) * 100.0;
    if pct_change.abs() <= threshold_pct {
        return;
    }

    let improvement = if higher_is_bad {
        pct_change < 0.0
    } else {
        pct_change > 0.0
    };
    push_delta_line(
        lines,
        label,
        baseline,
        current,
        pct_change,
        improvement,
        unit,
        "%",
    );
}

#[allow(clippy::too_many_arguments)]
fn push_delta_line(
    lines: &mut Vec<ComparisonLine>,
    label: &str,
    baseline: f64,
    current: f64,
    delta: f64,
    improvement: bool,
    unit: &str,
    delta_unit: &str,
) {
    let kind = if improvement {
        ComparisonKind::Improvement
    } else {
        ComparisonKind::Regression
    };
    let prefix = if improvement {
        "IMPROVEMENT"
    } else {
        "REGRESSION"
    };

    lines.push(ComparisonLine {
        kind,
        message: format!(
            "{prefix}: {label} {baseline:.2}{unit} -> {current:.2}{unit} ({delta:+.2} {delta_unit})"
        ),
    });
}

pub fn has_regressions(lines: &[ComparisonLine]) -> bool {
    lines
        .iter()
        .any(|line| line.kind == ComparisonKind::Regression)
}

pub fn format_comparison(lines: &[ComparisonLine]) -> Vec<String> {
    lines.iter().map(|line| line.message.clone()).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::stats::LatencySnapshot;

    fn snapshot(p99: f64, error_rate: f64, rps: f64) -> ReportSnapshot {
        ReportSnapshot {
            command: "hit".to_string(),
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            total_duration_secs: 1.0,
            completed: 100,
            successful: 100,
            failed: 0,
            error_rate,
            requests_per_sec: rps,
            latency: LatencySnapshot {
                min_ms: 1.0,
                p50_ms: 10.0,
                p90_ms: 20.0,
                p95_ms: 25.0,
                p99_ms: p99,
                max_ms: p99,
            },
            status_codes: BTreeMap::new(),
            errors: BTreeMap::new(),
            metadata: None,
        }
    }

    #[test]
    fn detects_p99_regression() {
        let baseline = snapshot(100.0, 0.0, 100.0);
        let current = snapshot(120.0, 0.0, 100.0);
        let lines = compare_reports(
            &baseline,
            &current,
            &ComparisonOptions {
                p99_threshold_pct: 10.0,
                error_rate_threshold_pts: 1.0,
            },
        );
        assert!(
            lines
                .iter()
                .any(|l| l.message.contains("REGRESSION: p99 latency"))
        );
    }

    #[test]
    fn detects_error_rate_regression() {
        let baseline = snapshot(50.0, 1.0, 100.0);
        let current = snapshot(50.0, 3.0, 100.0);
        let lines = compare_reports(
            &baseline,
            &current,
            &ComparisonOptions {
                p99_threshold_pct: 10.0,
                error_rate_threshold_pts: 1.0,
            },
        );
        assert!(
            lines
                .iter()
                .any(|l| l.message.contains("REGRESSION: error rate"))
        );
    }

    #[test]
    fn detects_rps_improvement() {
        let baseline = snapshot(50.0, 0.0, 100.0);
        let current = snapshot(50.0, 0.0, 130.0);
        let lines = compare_reports(
            &baseline,
            &current,
            &ComparisonOptions {
                p99_threshold_pct: 10.0,
                error_rate_threshold_pts: 1.0,
            },
        );
        assert!(
            lines
                .iter()
                .any(|l| l.message.contains("IMPROVEMENT: requests/sec"))
        );
    }
}
