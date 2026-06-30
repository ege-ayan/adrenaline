#!/usr/bin/env python3
"""Parse benchmark outputs into a comparison table."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

BENCH = Path(__file__).resolve().parent
RESULTS = BENCH / "bench-results"


def parse_adrenaline(path: Path) -> dict:
    data = json.loads(path.read_text())
    snap = data if "latency" in data else data.get("snapshot", data)
    if "latency" in snap and isinstance(snap["latency"], dict):
        lat = snap["latency"]
    else:
        lat = snap.get("latency", {})
    return {
        "rps": snap.get("requests_per_sec", snap.get("requestsPerSec", 0)),
        "p50": lat.get("p50_ms", lat.get("p50", 0)),
        "p95": lat.get("p95_ms", lat.get("p95", 0)),
        "p99": lat.get("p99_ms", lat.get("p99", 0)),
        "error_pct": snap.get("error_rate", snap.get("errorRate", 0)),
    }


def parse_locust_stats(path: Path) -> dict:
    lines = path.read_text().strip().splitlines()
    if len(lines) < 2:
        return {}
    header = lines[0].split(",")
    row = None
    for line in reversed(lines[1:]):
        parts = line.split(",")
        if len(parts) < len(header):
            continue
        data = dict(zip(header, parts))
        if data.get("Name") == "Aggregated" or (parts[0] == "" and parts[1] == "Aggregated"):
            row = data
            break
    if not row:
        return {}
    total = float(row.get("Request Count", 0) or 0)
    fails = float(row.get("Failure Count", 0) or 0)
    return {
        "rps": float(row.get("Requests/s", 0) or 0),
        "p50": float(row.get("50%", row.get("Median Response Time", 0)) or 0),
        "p95": float(row.get("95%", 0) or 0),
        "p99": float(row.get("99%", 0) or 0),
        "error_pct": (fails / total * 100) if total else 0,
        "completed": int(total),
    }


def parse_time_log(path: Path) -> dict:
    if not path.exists():
        return {"max_rss_mb": "?", "cpu_pct": "?"}
    text = path.read_text()
    max_rss_mb = "?"
    cpu_pct = "?"

    m = re.search(r"(\d+)\s+maximum resident set size", text)
    if m:
        max_rss_mb = f"{int(m.group(1)) / (1024 * 1024):.1f}"

    m = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    if m:
        max_rss_mb = f"{int(m.group(1)) / 1024:.1f}"

    m = re.search(r"Percent of CPU this job got:\s*(\d+)", text)
    if m:
        cpu_pct = m.group(1)
    else:
        # macOS: "13.86 real         3.87 user         2.63 sys"
        m = re.search(
            r"(\d+\.\d+)\s+real\s+(\d+\.\d+)\s+user\s+(\d+\.\d+)\s+sys", text
        )
        if m:
            r, u, s = float(m.group(1)), float(m.group(2)), float(m.group(3))
            if r > 0:
                cpu_pct = f"{(u + s) / r * 100:.0f}"
        else:
            real = re.search(r"(\d+\.\d+)\s+real", text)
            user = re.search(r"(\d+\.\d+)\s+user", text)
            sys_ = re.search(r"(\d+\.\d+)\s+sys", text)
            if real and user and sys_:
                r, u, s = float(real.group(1)), float(user.group(1)), float(sys_.group(1))
                if r > 0:
                    cpu_pct = f"{(u + s) / r * 100:.0f}"

    return {"max_rss_mb": max_rss_mb, "cpu_pct": cpu_pct}


def main() -> None:
    rows = []
    for c in (100, 500, 1000):
        for tool, prefix in (
            ("Adrenaline", f"adrenaline-c{c}"),
            ("Locust FastHttpUser", f"locust-c{c}"),
        ):
            if tool.startswith("Adrenaline"):
                json_path = RESULTS / f"{prefix}.json"
                if not json_path.exists():
                    continue
                metrics = parse_adrenaline(json_path)
            else:
                stats_path = RESULTS / f"{prefix}_stats.csv"
                if not stats_path.exists():
                    continue
                metrics = parse_locust_stats(stats_path)

            time_info = parse_time_log(RESULTS / f"{prefix}.time.log")
            rows.append(
                {
                    "tool": tool,
                    "concurrency": c,
                    **metrics,
                    **time_info,
                }
            )

    if not rows:
        print("No results found in", RESULTS, file=sys.stderr)
        sys.exit(1)

    print(
        "| Tool | Concurrency | RPS | p50 (ms) | p95 (ms) | p99 (ms) | Error % | Max RAM (MB) | CPU % |"
    )
    print("|------|-------------|-----|----------|----------|----------|---------|--------------|-------|")
    for r in rows:
        print(
            f"| {r['tool']} | {r['concurrency']} | "
            f"{r.get('rps', 0):.0f} | {r.get('p50', 0):.1f} | {r.get('p95', 0):.1f} | "
            f"{r.get('p99', 0):.1f} | {r.get('error_pct', 0):.2f} | "
            f"{r['max_rss_mb']} | {r['cpu_pct']} |"
        )


if __name__ == "__main__":
    main()
