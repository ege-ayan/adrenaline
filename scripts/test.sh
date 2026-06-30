#!/usr/bin/env bash
set -euo pipefail

output="$(cargo test --lib --tests --bins 2>&1)" || {
  echo "$output"
  exit 1
}

echo "$output"

passed=0
failed=0
ignored=0

while IFS= read -r line; do
  case "$line" in
    *"test result:"*)
      p=$(echo "$line" | sed -E 's/.*\. ([0-9]+) passed;.*/\1/')
      f=$(echo "$line" | sed -E 's/.*; ([0-9]+) failed;.*/\1/')
      i=$(echo "$line" | sed -E 's/.*; ([0-9]+) ignored;.*/\1/')
      passed=$((passed + p))
      failed=$((failed + f))
      ignored=$((ignored + i))
      ;;
  esac
done <<< "$output"

total=$((passed + failed + ignored))

echo ""
echo "────────────────────────────────────────"
if [[ $failed -eq 0 ]]; then
  echo "Total: ${total} tests — ${passed} passed, ${ignored} ignored"
else
  echo "Total: ${total} tests — ${passed} passed, ${failed} failed, ${ignored} ignored"
  exit 1
fi
