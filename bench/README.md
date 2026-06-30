# Adrenaline vs Locust (FastHttpUser)

Local nginx benchmarks on the same machine. Compare Adrenaline against Locust `FastHttpUser`.

## Quick start

```bash
make bench-setup    # docker nginx + Python venv + locust (once)
make bench          # max throughput test (~5 min)
make bench-rps      # same-RPS latency test (~7 min)
```

Results print as markdown tables. Raw files go to `bench/bench-results/` and `bench/bench-results-rps/` (gitignored).

### Makefile targets

| Target | Description |
|--------|-------------|
| `make bench-setup` | Start nginx container, create `bench/.venv`, install locust |
| `make bench-up` | Start nginx only (`BENCH_PORT=8088` by default) |
| `make bench-down` | Stop nginx container |
| `make bench` | Throughput benchmark + parse table |
| `make bench-rps` | Fixed RPS benchmark + parse table |
| `make bench-parse` | Re-print throughput results from disk |
| `make bench-parse-rps` | Re-print same-RPS results from disk |
| `make bench-clean` | Remove results, venv, stop container |

If port 8080/8088 is taken (e.g. another service on your Mac):

```bash
make bench-setup BENCH_PORT=8090
make bench BENCH_PORT=8090
```

## Manual setup

```bash
docker run -d --rm --name bench-nginx -p 8088:80 nginx:alpine
CARGO_TARGET_DIR=./target cargo build --release
python3 -m venv bench/.venv && bench/.venv/bin/pip install locust
./bench/run-benchmark.sh
python3 bench/parse-results.py
```

---

## Test 1: Max throughput (same concurrency)

```bash
make bench
```

| | Adrenaline | Locust FastHttpUser |
|---|------------|---------------------|
| Concurrency | `-c 100/500/1000` | `--users` + `--spawn-rate` (instant ramp) |
| Work load | Fixed requests (`-n 200k / 500k / 1M`) | Fixed time (`--run-time 30s`) |
| Wait time | back-to-back | `constant(0)` |

### Sample results (macOS, nginx:alpine @ :8088)

| Tool | Concurrency | RPS | p50 | p95 | p99 | Error % | Max RAM | CPU % |
|------|-------------|-----|-----|-----|-----|---------|---------|-------|
| Adrenaline | 100 | 14,429 | 4.1 | 17.6 | 55.3 | 0 | 22 MB | 47 |
| Locust | 100 | 8,510 | 8.0 | 13.0 | 21.0 | 0 | 69 MB | 82 |
| Adrenaline | 500 | 15,748 | 19.9 | 78.7 | 185.6 | 0 | 83 MB | 51 |
| Locust | 500 | 7,338 | 34.0 | 63.0 | 100.0 | 0 | 94 MB | 79 |
| Adrenaline | 1000 | 14,531 | 48.6 | 156.2 | 318.7 | 0 | 135 MB | 50 |
| Locust | 1000 | 6,258 | 73.0 | 140.0 | 260.0 | 0 | 122 MB | 74 |

**Takeaway:** Adrenaline pushes ~1.7–2.3× more RPS with less CPU at max load.

---

## Test 2: Same RPS (latency overhead)

Fix both tools to the same target RPS and compare p95/p99.

```bash
make bench-rps
```

| | Adrenaline | Locust FastHttpUser |
|---|------------|---------------------|
| Rate cap | `--rps 6000/8000/10000` | `constant_pacing(users / TARGET_RPS)` |
| Duration | 60s (`-n rps×60`) | `--run-time 60s` |
| Concurrency | `-c 400` | 150 / 200 / 350 users (scaled per tier) |

### Sample results (macOS, nginx:alpine @ :8088)

| Tool | Target RPS | Achieved RPS | p50 | p95 | p99 | Error % | Max RAM |
|------|------------|--------------|-----|-----|-----|---------|---------|
| Adrenaline | 6,000 | 5,556 | 1.9 | 80.9 | 202.5 | 0 | 64 MB |
| Locust | 6,000 | 5,216 | 11.0 | 29.0 | 54.0 | 0 | 69 MB |
| Adrenaline | 8,000 | 7,342 | 2.3 | 69.1 | 165.1 | 0 | 71 MB |
| Locust | 8,000 | 5,457 | 20.0 | 40.0 | 95.0 | 0 | 75 MB |
| Adrenaline | 10,000 | 8,820 | 3.5 | 69.6 | 166.8 | 0 | 66 MB |
| Locust | 10,000 | 4,973 | 34.0 | 75.0 | 130.0 | 0 | 79 MB |

*(latency in ms)*

**Caveats:**

- Compare **Achieved RPS**, not just target — Locust may cap below 8k/10k on some machines.
- For strict same-load comparison, pick a target both tools hit (e.g. 5000 RPS).

Re-run on your hardware; numbers vary by CPU, Docker, and OS.
