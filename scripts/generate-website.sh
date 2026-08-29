#!/usr/bin/env bash
# issues-26: compile website/generate.drac, emit HTML, stage to dist/pages.
set -euo pipefail

usage() {
  echo "Usage: scripts/generate-website.sh [--bin PATH] [--out DIR]" >&2
}

BIN=""
OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin)
      BIN="${2:-}"
      shift 2
      ;;
    --out)
      OUT="${2:-}"
      shift 2
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OUT:-"$ROOT/dist/pages"}"

if [[ -z "$BIN" ]]; then
  if [[ -n "${DRACONIC:-}" ]]; then
    BIN="$DRACONIC"
  elif [[ -f "$ROOT/target/release/draconic" ]]; then
    BIN="$ROOT/target/release/draconic"
  elif [[ -f "$ROOT/target/debug/draconic" ]]; then
    BIN="$ROOT/target/debug/draconic"
  else
    cargo build -p draconic-cli --release --manifest-path "$ROOT/Cargo.toml"
    BIN="$ROOT/target/release/draconic"
  fi
fi

if [[ ! -f "$BIN" ]]; then
  echo "binary not found: $BIN" >&2
  exit 1
fi

WORKDIR="$(mktemp -d)"
GEN="$WORKDIR/generate"
"$BIN" build --target native "$ROOT/website/generate.drac" -o "$GEN"
(cd "$ROOT" && "$GEN")

mkdir -p "$OUT"
shopt -s nullglob
htmls=("$ROOT"/website/*.html)
if [[ ${#htmls[@]} -eq 0 ]]; then
  echo "generator produced no HTML under website/" >&2
  exit 1
fi
cp "${htmls[@]}" "$OUT"/
if [[ ! -f "$OUT/learn.html" ]]; then
  echo "expected website/learn.html after generate" >&2
  exit 1
fi
cp "$OUT/learn.html" "$OUT/index.html"
touch "$OUT/.nojekyll"
echo "$OUT"
