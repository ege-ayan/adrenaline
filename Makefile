.PHONY: test fmt clippy build release install env \
	bench-setup bench-up bench-down bench bench-rps bench-parse bench-parse-rps bench-clean

BENCH_PORT ?= 8088
BENCH_CONTAINER ?= bench-nginx

test:
	@./scripts/test.sh

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build --release

install: build
	cargo install --path . --locked --force
	@echo ""
	@echo "Installed to ~/.cargo/bin/adrenaline"
	@echo "Ensure this is on your PATH (add to ~/.zshrc if needed):"
	@echo '  export PATH="$$HOME/.cargo/bin:$$PATH"'

env:
	@echo 'Run: source scripts/env.sh'
	@echo 'Or add to ~/.zshrc:'
	@echo '  export PATH="$$HOME/.cargo/bin:$$PATH"'

release:
	@chmod +x scripts/release.sh
	@./scripts/release.sh

# --- Benchmarks (Adrenaline vs Locust) ---
# Requires: docker, python3. See bench/README.md

bench-setup:
	@chmod +x scripts/bench-setup.sh bench/*.sh
	@BENCH_PORT=$(BENCH_PORT) BENCH_CONTAINER=$(BENCH_CONTAINER) ./scripts/bench-setup.sh

bench-up:
	@if docker ps --format '{{.Names}}' | grep -qx '$(BENCH_CONTAINER)'; then \
		echo "$(BENCH_CONTAINER) already running on port $(BENCH_PORT)"; \
	elif docker ps -a --format '{{.Names}}' | grep -qx '$(BENCH_CONTAINER)'; then \
		docker start $(BENCH_CONTAINER); \
	else \
		docker run -d --rm --name $(BENCH_CONTAINER) -p $(BENCH_PORT):80 nginx:alpine; \
	fi
	@echo "Target: http://127.0.0.1:$(BENCH_PORT)/"

bench-down:
	@docker stop $(BENCH_CONTAINER) 2>/dev/null || true

bench: bench-up
	@chmod +x bench/run-benchmark.sh
	@BENCH_PORT=$(BENCH_PORT) ./bench/run-benchmark.sh
	@$(MAKE) bench-parse

bench-rps: bench-up
	@chmod +x bench/run-rps-benchmark.sh
	@BENCH_PORT=$(BENCH_PORT) ./bench/run-rps-benchmark.sh
	@$(MAKE) bench-parse-rps

bench-parse:
	@python3 bench/parse-results.py

bench-parse-rps:
	@python3 bench/parse-rps-results.py

bench-clean: bench-down
	@rm -rf bench/bench-results bench/bench-results-rps bench/.venv bench/__pycache__
	@echo "Benchmark artifacts removed."
