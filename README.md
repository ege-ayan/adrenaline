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
| **HTTP methods** | GET, HEAD, POST, PUT, PATCH, DELETE, OPTIONS |
| **Headers & body** | Custom headers and request body from file |
| **JSON output** | `--json` for machine-readable results |
| **HTML reports** | `--html report.html` |
| **Baseline tracking** | `--save-baseline` / `--baseline` for CI regression checks |
| **GitHub Actions** | CI workflow with smoke load test |

## Requirements

**Pre-built release:** no extra dependencies — just download and run.

**Build from source:** [Rust](https://www.rust-lang.org/tools/install) 1.85+ (2024 edition)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

## Installation

### Pre-built binary (GitHub Releases)

1. Open [GitHub Releases](https://github.com/ege-ayan/adrenaline/releases/latest)
2. Download the archive for your platform (see table below)
3. Extract it and put `adrenaline` on your `PATH` (platform steps below)
4. Run `adrenaline --version` to confirm

| Platform | Download |
|----------|----------|
| Linux x86_64 | `adrenaline-*-linux-x86_64.tar.gz` |
| Linux ARM64 | `adrenaline-*-linux-aarch64.tar.gz` |
| macOS Apple Silicon | `adrenaline-*-macos-aarch64.tar.gz` |
| macOS Intel | `adrenaline-*-macos-x86_64.tar.gz` |
| Windows x86_64 | `adrenaline-*-windows-x86_64.zip` |

Replace `*` with the release version (e.g. `0.1.2`).

#### macOS (Apple Silicon or Intel)

```bash
cd ~/Downloads

# pick the file you downloaded, e.g.:
tar -xzf adrenaline-0.1.2-macos-aarch64.tar.gz

# install globally (requires password)
sudo install -m 755 adrenaline-0.1.2-macos-aarch64/adrenaline /usr/local/bin/adrenaline

# or install for your user only (no sudo)
mkdir -p ~/.local/bin
install -m 755 adrenaline-0.1.2-macos-aarch64/adrenaline ~/.local/bin/adrenaline
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

adrenaline --version
adrenaline hit https://example.com -n 10 -c 2
```

If macOS blocks the binary (“cannot be opened”), remove the quarantine flag once:

```bash
xattr -d com.apple.quarantine "$(which adrenaline)"
```

#### Linux

```bash
cd ~/Downloads

# example: linux x86_64
tar -xzf adrenaline-0.1.2-linux-x86_64.tar.gz

sudo install -m 755 adrenaline-0.1.2-linux-x86_64/adrenaline /usr/local/bin/adrenaline

# user-only install (no sudo)
mkdir -p ~/.local/bin
install -m 755 adrenaline-0.1.2-linux-x86_64/adrenaline ~/.local/bin/adrenaline
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

adrenaline --version
adrenaline hit https://example.com -n 10 -c 2
```

#### Windows (PowerShell)

```powershell
cd $env:USERPROFILE\Downloads

# example file name — adjust version if needed
Expand-Archive adrenaline-0.1.2-windows-x86_64.zip -DestinationPath .

# add to user PATH for this session
$bin = "$env:USERPROFILE\Downloads\adrenaline-0.1.2-windows-x86_64"
$env:Path = "$bin;$env:Path"

# persist PATH (open a new terminal after)
[Environment]::SetEnvironmentVariable(
  "Path",
  [Environment]::GetEnvironmentVariable("Path", "User") + ";$bin",
  "User"
)

adrenaline --version
adrenaline hit https://example.com -n 10 -c 2
```

#### Verify download (optional)

Each release includes `SHA256SUMS.txt`. On macOS/Linux:

```bash
shasum -a 256 -c SHA256SUMS.txt
```

On Windows (PowerShell), compare the hash manually:

```powershell
Get-FileHash adrenaline-0.1.2-windows-x86_64.zip -Algorithm SHA256
# match the line for your file in SHA256SUMS.txt
```

You should see `OK` (Unix) or matching hashes (Windows) for the archive you downloaded.

#### First run

```bash
adrenaline --help
adrenaline hit https://example.com -n 100 -c 10
```

### Build from source

```bash
git clone https://github.com/ege-ayan/adrenaline.git
cd adrenaline
cargo build --release
# run without installing: ./target/release/adrenaline hit ...
```

### Install to PATH

```bash
make install
```

This installs `adrenaline` to `~/.cargo/bin/`. If the command is not found, add Rust’s bin dir to your shell:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
adrenaline --version
```

**Local dev only** (no global install):

```bash
make build
source scripts/env.sh   # adds ./target/release to PATH for this shell
adrenaline --version
```

If you use [direnv](https://direnv.net), `.envrc` does the same when you `cd` into the repo.

### Package a local release binary

```bash
make release
# creates dist/adrenaline-<version>-<target>.tar.gz (or .zip on Windows)
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
| `--rps` | — | Cap request rate (requests per second) |
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
├── runner.rs        # Fixed worker pool load execution
├── stats.rs         # Worker-local stats, histogram merge, snapshots
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
├── ci.yml           # fmt, clippy, test, smoke load test
└── release.yml      # multi-platform release binaries
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

### Benchmarks (vs Locust)

Compare Adrenaline against Locust locally (requires Docker + Python 3):

```bash
make bench-setup    # once: nginx container + locust venv
make bench          # max throughput
make bench-rps      # fixed RPS / latency overhead
make bench-clean    # remove results and stop container
```

See [bench/README.md](bench/README.md) for methodology and sample numbers.

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
