#!/usr/bin/env bash
set -euo pipefail

if ! command -v hyperfine >/dev/null 2>&1; then
  echo "hyperfine not found" >&2
  exit 1
fi

PERF_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$PERF_DIR/../.." && pwd)"
DRACONIC="${DRACONIC:-$REPO/target/debug/draconic}"

if [[ ! -x "$DRACONIC" ]]; then
  cargo build -q -p draconic-cli --manifest-path "$REPO/Cargo.toml"
  DRACONIC="$REPO/target/debug/draconic"
fi

tmp="$(mktemp -d "${TMPDIR:-/tmp}/draconic-perf.XXXXXX")"
cleanup() {
  rm -rf "$tmp"
}
trap cleanup EXIT

bench_one() {
  local src="$1"
  local target="$2"
  local stem out compile_cmd run_cmd
  stem="$(basename "$src" .drac)"
  if [[ "$target" == js ]]; then
    out="$tmp/${stem}.js"
    run_cmd="node \"$out\""
  else
    out="$tmp/${stem}"
    run_cmd="\"$out\""
  fi
  compile_cmd="\"$DRACONIC\" build --target $target \"$src\" -o \"$out\""
  hyperfine --command-name "${stem} ${target} compile" "$compile_cmd"
  hyperfine --command-name "${stem} ${target} run" "$run_cmd"
}

for src in "$PERF_DIR/compile_heavy.drac" "$PERF_DIR/run_heavy.drac"; do
  for target in js native; do
    bench_one "$src" "$target"
  done
done
