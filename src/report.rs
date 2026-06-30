use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;

use crate::baseline::{ComparisonOptions, compare_reports, format_comparison, has_regressions};
use crate::cli::OutputArgs;
use crate::output::{
    load_baseline, print_comparison, render_report, write_baseline, write_html_report,
};
use crate::stats::TestReport;

pub fn finish_report(report: &TestReport, output: &OutputArgs) -> Result<ExitCode> {
    println!("{}", render_report(report, output.json));

    if let Some(path) = &output.html {
        write_html_report(report, path)?;
        eprintln!("HTML report written to {}", path.display());
    }

    if let Some(path) = &output.save_baseline {
        write_baseline(report, path)?;
        eprintln!("Baseline saved to {}", path.display());
    }

    if let Some(path) = &output.baseline {
        let baseline = load_baseline(path)?;
        let lines = compare_reports(
            &baseline,
            &report.snapshot(),
            &ComparisonOptions {
                p99_threshold_pct: 10.0,
                error_rate_threshold_pts: 1.0,
            },
        );
        print_comparison(&format_comparison(&lines));
        if has_regressions(&lines) {
            return Ok(ExitCode::from(1));
        }
    }

    Ok(ExitCode::SUCCESS)
}

pub fn metadata_pair(key: &str, value: impl ToString) -> (String, String) {
    (key.to_string(), value.to_string())
}

pub fn metadata_map(pairs: &[(&str, String)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

pub fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
