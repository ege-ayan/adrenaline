#!/usr/bin/env bash
# Shared helpers for bench/run-*.sh (sourced, not executed directly).

bench_root() {
  cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd
}

bench_init() {
  ROOT="$(bench_root)"
  BENCH="$ROOT/bench"
  BENCH_PORT="${BENCH_PORT:-8088}"
  TARGET="${BENCH_TARGET:-http://127.0.0.1:${BENCH_PORT}/}"
  RESULTS="${BENCH_RESULTS:-$BENCH/bench-results}"
  ADRENALINE="$ROOT/target/release/adrenaline"
  VENV="$BENCH/.venv"
  mkdir -p "$RESULTS"
}

bench_build_adrenaline() {
  echo "Building adrenaline..."
  (cd "$ROOT" && CARGO_TARGET_DIR=./target cargo build --release)
}

bench_setup_venv() {
  if [[ ! -d "$VENV" ]]; then
    echo "Creating Python venv and installing locust..."
    python3 -m venv "$VENV"
    "$VENV/bin/pip" install -q --upgrade pip
    "$VENV/bin/pip" install -q locust
  fi
}

bench_require_target() {
  if ! curl -sf -o /dev/null "$TARGET"; then
    echo "Benchmark target not reachable: $TARGET" >&2
    echo "Start it with: make bench-up" >&2
    exit 1
  fi
}

bench_run_timed() {
  local label="$1"
  shift
  local log="$RESULTS/${label}.time.log"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    /usr/bin/time -l "$@" >"$RESULTS/${label}.stdout.log" 2>"$log" || true
  elif command -v /usr/bin/time >/dev/null 2>&1; then
    /usr/bin/time -v "$@" >"$RESULTS/${label}.stdout.log" 2>"$log" || true
  else
    "$@" >"$RESULTS/${label}.stdout.log" 2>"$log" || true
  fi
}
