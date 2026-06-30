.PHONY: test fmt clippy build release install env

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
