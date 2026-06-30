#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Use project-local target/ (ignore Cursor/sandbox CARGO_TARGET_DIR).
export CARGO_TARGET_DIR="${root}/target"

host="$(rustc -vV | sed -n 's/host: //p')"
target="${1:-$host}"
version="$(awk -F'"' '/^version/ {print $2; exit}' Cargo.toml)"
stem="adrenaline-${version}-${target}"
bin_name="adrenaline"

echo "Building ${bin_name} ${version} for ${target}..."

if command -v rustup >/dev/null; then
  if ! rustup target list --installed | grep -q "^${target}\$"; then
    rustup target add "${target}"
  fi
fi

cargo build --release --target "${target}"

bin_path="${CARGO_TARGET_DIR}/${target}/release/${bin_name}"
if [[ "${target}" == *"windows"* ]]; then
  bin_path="${bin_path}.exe"
fi

if [[ ! -f "${bin_path}" ]]; then
  echo "error: binary not found at ${bin_path}" >&2
  exit 1
fi

rm -rf "dist/${stem}" "dist/${stem}.tar.gz" "dist/${stem}.zip"
mkdir -p "dist/${stem}"

if [[ "${target}" == *"windows"* ]]; then
  cp "${bin_path}" "dist/${stem}/${bin_name}.exe"
  (cd dist && zip -r "${stem}.zip" "${stem}")
  echo "Created dist/${stem}.zip"
  ls -lh "dist/${stem}.zip"
else
  cp "${bin_path}" "dist/${stem}/${bin_name}"
  chmod +x "dist/${stem}/${bin_name}"
  (cd dist && tar -czf "${stem}.tar.gz" "${stem}")
  echo "Created dist/${stem}.tar.gz"
  ls -lh "dist/${stem}.tar.gz"
fi
