#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
checker="$repo_root/scripts/check-rust-file-sizes.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/config" "$fixture/crates/example/src"
printf 'example\tcrates/example/\t3\t5\n' > "$fixture/config/rust-file-size-budgets.tsv"
printf 'crates/example/src/legacy.rs\t7\t#999\n' > "$fixture/config/rust-file-size-baseline.tsv"

write_lines() {
  local path="$1"
  local count="$2"
  awk -v count="$count" 'BEGIN { for (i = 0; i < count; i++) print "// line" }' > "$path"
}

write_lines "$fixture/crates/example/src/legacy.rs" 7
write_lines "$fixture/crates/example/src/warning.rs" 4
"$checker" --root "$fixture" --top 2 >/dev/null

write_lines "$fixture/crates/example/src/new.rs" 6
if "$checker" --root "$fixture" >/dev/null 2>&1; then
  echo "expected an unbaselined hard-limit violation" >&2
  exit 1
fi
rm "$fixture/crates/example/src/new.rs"

write_lines "$fixture/crates/example/src/legacy.rs" 8
if "$checker" --root "$fixture" >/dev/null 2>&1; then
  echo "expected growth beyond a baseline to fail" >&2
  exit 1
fi

echo "Rust file size checker tests passed."
