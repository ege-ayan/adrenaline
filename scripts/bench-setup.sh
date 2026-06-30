#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BENCH_PORT="${BENCH_PORT:-8088}"
CONTAINER="${BENCH_CONTAINER:-bench-nginx}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd docker
require_cmd python3
require_cmd curl

# Python venv + locust
VENV="$ROOT/bench/.venv"
if [[ ! -d "$VENV" ]]; then
  echo "Creating bench Python venv..."
  python3 -m venv "$VENV"
fi
"$VENV/bin/pip" install -q --upgrade pip
"$VENV/bin/pip" install -q locust
echo "Locust: $("$VENV/bin/locust" --version)"

# nginx target
if docker ps --format '{{.Names}}' | grep -qx "$CONTAINER"; then
  echo "Benchmark nginx already running on port $BENCH_PORT"
elif docker ps -a --format '{{.Names}}' | grep -qx "$CONTAINER"; then
  docker start "$CONTAINER" >/dev/null
  echo "Started existing container $CONTAINER on port $BENCH_PORT"
else
  docker run -d --rm --name "$CONTAINER" -p "${BENCH_PORT}:80" nginx:alpine >/dev/null
  echo "Started $CONTAINER on port $BENCH_PORT"
fi

sleep 1
if curl -sf -o /dev/null "http://127.0.0.1:${BENCH_PORT}/"; then
  echo "Target ready: http://127.0.0.1:${BENCH_PORT}/"
else
  echo "Warning: target not responding on port $BENCH_PORT" >&2
  echo "Port may be in use — try: make bench-up BENCH_PORT=8090" >&2
  exit 1
fi

echo ""
echo "Setup complete. Run:"
echo "  make bench       # max throughput vs Locust"
echo "  make bench-rps   # fixed RPS comparison"
