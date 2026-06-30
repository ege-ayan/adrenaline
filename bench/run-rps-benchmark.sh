#!/usr/bin/env bash
set -euo pipefail

# shellcheck source=common.sh
source "$(dirname "$0")/common.sh"

bench_init
BENCH_RESULTS="$BENCH/bench-results-rps"
RESULTS="$BENCH_RESULTS"
mkdir -p "$RESULTS"

CONCURRENCY=400
DURATION_SECS=60
RPS_CONFIG=(
  "6000:150"
  "8000:200"
  "10000:350"
)

bench_build_adrenaline
bench_setup_venv
bench_require_target

LOCUST="$VENV/bin/locust"

for entry in "${RPS_CONFIG[@]}"; do
  rps="${entry%%:*}"
  users="${entry##*:}"
  requests=$((rps * DURATION_SECS))
  echo "=== Adrenaline ${rps} RPS (${requests} requests, ${DURATION_SECS}s, c=${CONCURRENCY}) ==="
  bench_run_timed "adrenaline-rps${rps}" \
    "$ADRENALINE" hit "$TARGET" \
    -n "$requests" \
    -c "$CONCURRENCY" \
    --rps "$rps" \
    --save-baseline "$RESULTS/adrenaline-rps${rps}.json"

  echo "=== Locust ${rps} RPS (${users} users, ${DURATION_SECS}s) ==="
  bench_run_timed "locust-rps${rps}" \
    env TARGET_RPS="$rps" LOCUST_USERS="$users" \
    "$LOCUST" -f "$BENCH/locustfile_rps.py" \
    --headless \
    --host "$TARGET" \
    --users "$users" \
    --spawn-rate "$users" \
    --run-time "${DURATION_SECS}s" \
    --csv "$RESULTS/locust-rps${rps}"
done

echo "Done. Results in $RESULTS"
echo "Run: make bench-parse-rps"
