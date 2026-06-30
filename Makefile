.PHONY: test fmt clippy build

test:
	@./scripts/test.sh

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

build:
	cargo build --release
