#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=common.sh
source "$(dirname "$0")/common.sh"

bench_init
BENCH_RESULTS="$BENCH/bench-results"
RESULTS="$BENCH_RESULTS"
mkdir -p "$RESULTS"

bench_build_adrenaline
bench_setup_venv
bench_require_target

LOCUST="$VENV/bin/locust"

echo "=== Adrenaline c100 ==="
bench_run_timed adrenaline-c100 "$ADRENALINE" hit "$TARGET" -n 200000 -c 100 --json --save-baseline "$RESULTS/adrenaline-c100.json"

echo "=== Adrenaline c500 ==="
bench_run_timed adrenaline-c500 "$ADRENALINE" hit "$TARGET" -n 500000 -c 500 --json --save-baseline "$RESULTS/adrenaline-c500.json"

echo "=== Adrenaline c1000 ==="
bench_run_timed adrenaline-c1000 "$ADRENALINE" hit "$TARGET" -n 1000000 -c 1000 --json --save-baseline "$RESULTS/adrenaline-c1000.json"

echo "=== Locust c100 ==="
bench_run_timed locust-c100 "$LOCUST" -f "$BENCH/locustfile.py" --headless --host "$TARGET" --users 100 --spawn-rate 100 --run-time 30s --csv "$RESULTS/locust-c100"

echo "=== Locust c500 ==="
bench_run_timed locust-c500 "$LOCUST" -f "$BENCH/locustfile.py" --headless --host "$TARGET" --users 500 --spawn-rate 500 --run-time 30s --csv "$RESULTS/locust-c500"

echo "=== Locust c1000 ==="
bench_run_timed locust-c1000 "$LOCUST" -f "$BENCH/locustfile.py" --headless --host "$TARGET" --users 1000 --spawn-rate 1000 --run-time 30s --csv "$RESULTS/locust-c1000"

echo "Done. Results in $RESULTS"
echo "Run: make bench-parse"
