#!/usr/bin/env python3
"""Parse same-RPS benchmark results."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

BENCH = Path(__file__).resolve().parent
RESULTS = BENCH / "bench-results-rps"
RPS_LEVELS = (6000, 8000, 10000)


def parse_adrenaline(path: Path) -> dict:
    data = json.loads(path.read_text())
    lat = data["latency"]
    return {
        "target_rps": int(data.get("metadata", {}).get("rps", 0)),
        "achieved_rps": data["requests_per_sec"],
        "p50": lat["p50_ms"],
        "p95": lat["p95_ms"],
        "p99": lat["p99_ms"],
        "error_pct": data["error_rate"],
        "duration_secs": data["total_duration_secs"],
    }


def parse_locust_stats(path: Path) -> dict:
    lines = path.read_text().strip().splitlines()
    header = lines[0].split(",")
    for line in reversed(lines[1:]):
        parts = line.split(",")
        if len(parts) < len(header):
            continue
        row = dict(zip(header, parts))
        if row.get("Name") == "Aggregated":
            total = float(row["Request Count"])
            fails = float(row["Failure Count"])
            return {
                "achieved_rps": float(row["Requests/s"]),
                "p50": float(row["50%"]),
                "p95": float(row["95%"]),
                "p99": float(row["99%"]),
                "error_pct": (fails / total * 100) if total else 0,
                "completed": int(total),
            }
    return {}


def parse_time_log(path: Path) -> str:
    if not path.exists():
        return "?"
    text = path.read_text()
    m = re.search(r"(\d+)\s+maximum resident set size", text)
    if m:
        return f"{int(m.group(1)) / (1024 * 1024):.1f}"
    m = re.search(r"Maximum resident set size \(kbytes\):\s*(\d+)", text)
    if m:
        return f"{int(m.group(1)) / 1024:.1f}"
    return "?"


def main() -> None:
    if not RESULTS.exists():
        print(f"Missing {RESULTS}. Run ./bench/run-rps-benchmark.sh first.", file=sys.stderr)
        sys.exit(1)

    print(
        "| Tool | Target RPS | Achieved RPS | p50 (ms) | p95 (ms) | p99 (ms) | Error % | Max RAM (MB) |"
    )
    print(
        "|------|------------|--------------|----------|----------|----------|---------|--------------|"
    )

    for rps in RPS_LEVELS:
        for tool, prefix in (
            ("Adrenaline", f"adrenaline-rps{rps}"),
            ("Locust FastHttpUser", f"locust-rps{rps}"),
        ):
            if tool.startswith("Adrenaline"):
                path = RESULTS / f"{prefix}.json"
                if not path.exists():
                    continue
                m = parse_adrenaline(path)
                target = m.get("target_rps") or rps
            else:
                path = RESULTS / f"{prefix}_stats.csv"
                if not path.exists():
                    continue
                m = parse_locust_stats(path)
                target = rps

            ram = parse_time_log(RESULTS / f"{prefix}.time.log")
            print(
                f"| {tool} | {target:,} | {m.get('achieved_rps', 0):,.0f} | "
                f"{m.get('p50', 0):.1f} | {m.get('p95', 0):.1f} | {m.get('p99', 0):.1f} | "
                f"{m.get('error_pct', 0):.2f} | {ram} |"
            )


if __name__ == "__main__":
    main()
