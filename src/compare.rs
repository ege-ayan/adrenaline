use std::path::Path;
use std::process::ExitCode;

use anyhow::{Result, bail};

use crate::baseline::{ComparisonOptions, compare_reports, format_comparison, has_regressions};
use crate::cli::CompareArgs;
use crate::output::{load_baseline, print_comparison};

pub fn run(args: CompareArgs) -> Result<ExitCode> {
    let baseline = load_baseline(&args.baseline)?;
    let current = load_baseline(&args.current)?;

    let lines = compare_reports(
        &baseline,
        &current,
        &ComparisonOptions {
            p99_threshold_pct: args.p99_threshold,
            error_rate_threshold_pts: args.error_rate_threshold,
        },
    );

    print_comparison(&format_comparison(&lines));

    if has_regressions(&lines) {
        Ok(ExitCode::from(1))
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

pub fn compare_files(
    baseline_path: &Path,
    current_path: &Path,
    options: &ComparisonOptions,
) -> Result<(Vec<String>, bool)> {
    let baseline = load_baseline(baseline_path)?;
    let current = load_baseline(current_path)?;

    if baseline.command != current.command {
        bail!(
            "command mismatch: {} vs {}",
            baseline.command,
            current.command
        );
    }

    let lines = compare_reports(&baseline, &current, options);
    let failed = has_regressions(&lines);
    Ok((format_comparison(&lines), failed))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::NamedTempFile;

    use super::*;
    use crate::stats::{LatencySnapshot, ReportSnapshot};

    fn write_snapshot(path: &Path, p99: f64, error_rate: f64) {
        let snap = ReportSnapshot {
            command: "hit".to_string(),
            url: "https://example.com".to_string(),
            method: "GET".to_string(),
            total_duration_secs: 1.0,
            completed: 100,
            successful: 100,
            failed: 0,
            error_rate,
            requests_per_sec: 100.0,
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
        };
        std::fs::write(path, serde_json::to_string_pretty(&snap).unwrap()).unwrap();
    }

    #[test]
    fn compare_files_detects_regression_exit() {
        let baseline = NamedTempFile::new().unwrap();
        let current = NamedTempFile::new().unwrap();
        write_snapshot(baseline.path(), 100.0, 0.0);
        write_snapshot(current.path(), 150.0, 0.0);

        let (lines, failed) = compare_files(
            baseline.path(),
            current.path(),
            &ComparisonOptions {
                p99_threshold_pct: 10.0,
                error_rate_threshold_pts: 1.0,
            },
        )
        .unwrap();

        assert!(failed);
        assert!(lines.iter().any(|l| l.contains("REGRESSION")));
    }
}
