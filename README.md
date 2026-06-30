# Adrenaline

A lightweight, fast CLI load and stress testing tool written in Rust.

Adrenaline sends concurrent HTTP requests to API endpoints and reports latency percentiles, throughput, error rates, and status code distribution — from your terminal or CI pipeline.

```bash
adrenaline hit https://example.com -n 1000 -c 50
adrenaline ramp https://example.com -n 1000 --start-concurrency 5 --end-concurrency 100
adrenaline scenario examples/scenario.yaml
```

## Features

| Feature | Description |
|---------|-------------|
| **hit** | Fixed concurrency load test |
| **ramp** | Gradually increase concurrency across steps |
| **spike** | Baseline load followed by a sudden traffic burst |
| **find-limit** | Step up concurrency until error rate exceeds threshold |
| **compare** | Compare two baseline JSON reports for regressions |
| **scenario** | Run multi-step YAML test plans |
| **HTTP methods** | GET, POST, PUT, DELETE |
| **Headers & body** | Custom headers and request body from file |
| **JSON output** | `--json` for machine-readable results |
| **HTML reports** | `--html report.html` |
| **Baseline tracking** | `--save-baseline` / `--baseline` for CI regression checks |
| **GitHub Actions** | CI workflow with smoke load test |

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85+ (2024 edition)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

## Installation

```bash
git clone https://github.com/ege-ayan/adrenaline.git
cd adrenaline
cargo build --release
# binary: ./target/release/adrenaline

cargo install --path .
```

## Commands

### `hit` — constant load

```bash
adrenaline hit https://api.example.com/health \
  -n 1000 \
  -c 50 \
  --method GET \
  --header "Authorization: Bearer token" \
  --body payload.json \
  --timeout 10 \
  --json \
  --html report.html \
  --save-baseline baseline.json
```

| Flag | Default | Description |
|------|---------|-------------|
| `-n, --requests` | 1000 | Total requests |
| `-c, --concurrency` | 50 | Max in-flight requests |
| `--method` | GET | HTTP method |
| `--header` | — | Repeatable `Key: Value` header |
| `--body` | — | Request body file (POST/PUT) |
| `--timeout` | 10 | Per-request timeout (seconds) |
| `--json` | false | JSON output |
| `--html` | — | Write HTML report |
| `--baseline` | — | Compare against saved baseline |
| `--save-baseline` | — | Save results as baseline JSON |

### `ramp` — gradual load increase

```bash
adrenaline ramp https://api.example.com -n 1000 \
  --start-concurrency 5 \
  --end-concurrency 100 \
  --steps 10
```

Distributes total requests across `--steps`, linearly increasing concurrency from start to end.

### `spike` — traffic burst

```bash
adrenaline spike https://api.example.com \
  --baseline-requests 100 --baseline-concurrency 10 \
  --spike-requests 500 --spike-concurrency 200
```

Runs baseline phase first, then a high-concurrency spike, and reports combined metrics.

### `find-limit` — discover concurrency ceiling

```bash
adrenaline find-limit https://api.example.com \
  --requests-per-step 100 \
  --start-concurrency 10 \
  --max-concurrency 500 \
  --step 20 \
  --max-error-rate 5.0
```

Steps concurrency until error rate exceeds `--max-error-rate`. Reports the last passing concurrency as **limit found**.

### `compare` — baseline regression check

```bash
adrenaline compare baseline.json current.json \
  --p99-threshold 10 \
  --error-rate-threshold 1
```

Exits with code **1** if regressions are detected (useful in CI).

Compared metrics: error rate, p99 latency, p50 latency, requests/sec.

### `scenario` — YAML test plans

```bash
adrenaline scenario examples/scenario.yaml --json
```

Example scenario (`examples/scenario.yaml`):

```yaml
name: api smoke test
defaults:
  timeout: 10
  method: GET
steps:
  - type: hit
    url: https://example.com
    requests: 50
    concurrency: 5
  - type: ramp
    url: https://example.com
    requests: 200
    start_concurrency: 5
    end_concurrency: 50
    steps: 5
  - type: spike
    url: https://example.com
    baseline_requests: 50
    baseline_concurrency: 5
    spike_requests: 200
    spike_concurrency: 100
  - type: find-limit
    url: https://example.com
    requests_per_step: 50
    start_concurrency: 10
    max_concurrency: 200
    step: 20
    max_error_rate: 5.0
```

## Example output

```
Adrenaline hit

Target:       https://example.com
Method:       GET
Requests:     1000
Concurrency:  50
Timeout:      10s

Summary
-------
Total time:   2.41s
Requests/sec: 414.93
Completed:    1000
Successful:   998
Failed:       2
Error rate:   0.20%

Latency
-------
min:  12ms
p50:  34ms
p90:  81ms
p95:  120ms
p99:  310ms
max:  590ms

Status codes
------------
200: 998
500: 2
```

JSON output (`--json`) serializes the same data as `ReportSnapshot` for tooling and baselines.

## Project structure

```
src/
├── main.rs          # Binary entry point
├── lib.rs           # Library + command dispatch
├── cli.rs           # clap structs and validation
├── request.rs       # HTTP method, headers, send_request
├── runner.rs        # Semaphore-based load execution
├── stats.rs         # Histogram, aggregation, snapshots
├── output.rs        # Text, JSON, HTML formatting
├── baseline.rs      # Baseline comparison logic
├── report.rs        # Output orchestration (baseline, HTML)
├── hit.rs           # hit command
├── ramp.rs          # ramp command
├── spike.rs         # spike command
├── find_limit.rs    # find-limit command
├── compare.rs       # compare command
└── scenario.rs      # YAML scenario runner

tests/
└── cli_integration.rs   # End-to-end CLI tests (wiremock)

examples/
└── scenario.yaml

.github/workflows/
└── ci.yml           # fmt, clippy, test, smoke load test
```

## Development

```bash
cargo build
make test                     # runs all tests + prints total summary
./scripts/test.sh             # same as make test
cargo test                    # plain cargo (no total line at the end)
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo run -- hit https://example.com -n 100 -c 10
```

`make test` ends with a summary like `Total: 41 tests — 41 passed, 0 ignored`.
Plain `cargo test` runs the same tests but prints per-crate blocks without a grand total.

### Test coverage

| Area | Tests |
|------|-------|
| CLI validation | Zero requests/concurrency, invalid URL |
| HTTP layer | Methods, headers, body file, timeout classification |
| Stats | Success/failure, histogram merge, JSON snapshots |
| Output | Text, JSON, HTML rendering |
| Baseline | Regression/improvement detection |
| Commands | hit, ramp, spike, find-limit via wiremock |
| Scenario | YAML parsing and execution |
| Integration | Full CLI via `assert_cmd` |

## GitHub Actions

The included workflow (`.github/workflows/ci.yml`) runs:

1. `cargo fmt --check`
2. `cargo clippy`
3. `cargo test`
4. Release smoke test against `https://example.com`
5. Baseline artifact upload

Use in your own pipeline:

```yaml
- name: Load test
  run: |
    cargo run --release -- hit ${{ env.API_URL }} -n 200 -c 20 --json --save-baseline baseline.json

- name: Compare against baseline
  run: |
    cargo run --release -- compare previous-baseline.json baseline.json
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| clap | CLI parsing |
| tokio | Async runtime |
| reqwest | HTTP client (rustls) |
| hdrhistogram | Latency percentiles |
| serde / serde_json / serde_yaml | JSON & YAML |
| anyhow | Error handling |

## License

GPL-3.0 — see [LICENSE](LICENSE).
