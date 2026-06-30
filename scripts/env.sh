#!/usr/bin/env bash
# Usage: source scripts/env.sh
#
# Puts the local release binary and cargo-installed tools on PATH for this shell.

_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

export PATH="${_root}/target/release:${HOME}/.cargo/bin:${PATH}"

if [[ -x "${_root}/target/release/adrenaline" ]]; then
  echo "adrenaline -> ${_root}/target/release/adrenaline"
elif command -v adrenaline >/dev/null; then
  echo "adrenaline -> $(command -v adrenaline)"
else
  echo "adrenaline not built yet — run: make build" >&2
fi
