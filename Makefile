.PHONY: test fmt clippy build release

test:
	@./scripts/test.sh

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build --release

release:
	@chmod +x scripts/release.sh
	@./scripts/release.sh
