#!/usr/bin/env bash
set -euo pipefail

if [[ ! -t 0 || ! -t 1 ]]; then
  echo "This script must run in an interactive terminal." >&2
  exit 1
fi

if ! command -v stty >/dev/null 2>&1; then
  echo "stty is required." >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

check_stty_restored() {
  local label="$1"
  local before="$2"
  local after
  after="$(stty -g)"
  if [[ "$before" != "$after" ]]; then
    echo "FAIL: terminal mode changed after $label" >&2
    echo "before: $before" >&2
    echo "after:  $after" >&2
    exit 1
  fi
  echo "OK: $label restored terminal mode"
}

echo "Building salieri..."
cargo build --quiet

echo
echo "1. Normal exit: Salieri will open. Press q to quit."
before="$(stty -g)"
target/debug/salieri
check_stty_restored "normal exit" "$before"

echo
echo "2. Panic recovery: Salieri will intentionally panic after terminal setup."
before="$(stty -g)"
set +e
SALIERI_DEBUG_PANIC_AFTER_TERMINAL_ENTER=1 target/debug/salieri >/tmp/salieri-terminal-panic.log 2>&1
status=$?
set -e
if [[ "$status" -eq 0 ]]; then
  echo "FAIL: debug panic command exited successfully" >&2
  exit 1
fi
check_stty_restored "panic" "$before"

echo
echo "3. SIGINT recovery: Salieri will open and receive external SIGINT."
before="$(stty -g)"
target/debug/salieri &
pid=$!
sleep 1
kill -INT "$pid"
wait "$pid" || true
check_stty_restored "SIGINT" "$before"

echo
echo "Terminal recovery verification passed."
