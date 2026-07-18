#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scan_root="$repo_root"
top_count=10

usage() {
  cat <<'EOF'
Usage: scripts/check-rust-file-sizes.sh [--root PATH] [--top COUNT]

Checks Rust source files against domain budgets and recorded exceptions.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root)
      [[ $# -ge 2 ]] || { echo "--root requires a path" >&2; exit 2; }
      scan_root="$(cd "$2" && pwd)"
      shift 2
      ;;
    --top)
      [[ $# -ge 2 && "$2" =~ ^[0-9]+$ && "$2" -gt 0 ]] || {
        echo "--top requires a positive integer" >&2
        exit 2
      }
      top_count="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

budget_file="$scan_root/config/rust-file-size-budgets.tsv"
baseline_file="$scan_root/config/rust-file-size-baseline.tsv"

for required_file in "$budget_file" "$baseline_file"; do
  if [[ ! -f "$required_file" ]]; then
    echo "missing configuration: ${required_file#"$scan_root"/}" >&2
    exit 2
  fi
done

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
counts_file="$tmp_dir/counts.tsv"

while IFS= read -r -d '' file; do
  relative_path="${file#"$scan_root"/}"
  lines="$(wc -l < "$file")"
  lines="${lines//[[:space:]]/}"
  printf '%s\t%s\n' "$lines" "$relative_path" >> "$counts_file"
done < <(find "$scan_root/crates" -type f -name '*.rs' -not -path '*/target/*' -print0)

if [[ ! -s "$counts_file" ]]; then
  echo "no Rust source files found under crates/" >&2
  exit 2
fi

echo "Largest Rust source files:"
sort -t $'\t' -k1,1nr -k2,2 "$counts_file" | sed -n "1,${top_count}p" | while IFS=$'\t' read -r lines path; do
  printf '  %6d  %s\n' "$lines" "$path"
done

failures=0
warnings=0

while IFS=$'\t' read -r lines path; do
  domain=""
  soft_limit=""
  hard_limit=""
  best_prefix_length=-1

  while IFS=$'\t' read -r candidate_domain prefix candidate_soft candidate_hard; do
    [[ -n "$candidate_domain" && "${candidate_domain:0:1}" != "#" ]] || continue
    if [[ "$path" == "$prefix"* && ${#prefix} -gt $best_prefix_length ]]; then
      domain="$candidate_domain"
      soft_limit="$candidate_soft"
      hard_limit="$candidate_hard"
      best_prefix_length=${#prefix}
    fi
  done < "$budget_file"

  if [[ -z "$domain" ]]; then
    echo "FAIL: $path has no matching domain budget" >&2
    failures=$((failures + 1))
    continue
  fi

  baseline_limit=""
  tracking_issue=""
  while IFS=$'\t' read -r baseline_path candidate_limit candidate_issue; do
    [[ -n "$baseline_path" && "${baseline_path:0:1}" != "#" ]] || continue
    if [[ "$path" == "$baseline_path" ]]; then
      baseline_limit="$candidate_limit"
      tracking_issue="$candidate_issue"
      break
    fi
  done < "$baseline_file"

  if [[ -n "$baseline_limit" ]]; then
    if [[ -z "$tracking_issue" ]]; then
      echo "FAIL: $path has an undocumented baseline exception" >&2
      failures=$((failures + 1))
    elif (( lines > baseline_limit )); then
      echo "FAIL: $path has $lines lines; baseline is $baseline_limit ($tracking_issue)" >&2
      failures=$((failures + 1))
    elif (( lines > hard_limit )); then
      echo "BASELINED: $path has $lines lines; hard limit is $hard_limit ($tracking_issue)"
    fi
  elif (( lines > hard_limit )); then
    echo "FAIL: $path has $lines lines; $domain hard limit is $hard_limit" >&2
    failures=$((failures + 1))
  elif (( lines > soft_limit )); then
    echo "WARN: $path has $lines lines; $domain soft limit is $soft_limit"
    warnings=$((warnings + 1))
  fi
done < "$counts_file"

while IFS=$'\t' read -r baseline_path baseline_limit tracking_issue; do
  [[ -n "$baseline_path" && "${baseline_path:0:1}" != "#" ]] || continue
  if [[ ! -f "$scan_root/$baseline_path" ]]; then
    echo "FAIL: stale baseline entry for missing file $baseline_path" >&2
    failures=$((failures + 1))
  elif [[ ! "$baseline_limit" =~ ^[0-9]+$ || -z "$tracking_issue" ]]; then
    echo "FAIL: invalid baseline entry for $baseline_path" >&2
    failures=$((failures + 1))
  fi
done < "$baseline_file"

if (( failures > 0 )); then
  echo "Rust file size check failed with $failures violation(s)." >&2
  exit 1
fi

echo "Rust file size check passed with $warnings warning(s)."
