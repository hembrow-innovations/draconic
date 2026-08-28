#!/usr/bin/env bash
# D01.01: stage the draconic CLI as dist/draconic-<host-triple>.
set -euo pipefail

usage() {
  echo "Usage: scripts/release-artifact.sh [--bin PATH] [--out DIR]" >&2
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
OUT="${OUT:-"$ROOT/dist"}"

host_triple() {
  local info line
  info="$(rustc -vV)"
  while IFS= read -r line; do
    case "$line" in
      "host: "*)
        printf '%s\n' "${line#host: }"
        return 0
        ;;
    esac
  done <<< "$info"
  return 1
}

TRIPLE="$(host_triple)"
if [[ -z "$TRIPLE" ]]; then
  echo "could not detect host triple from rustc -vV" >&2
  exit 1
fi

if [[ -z "$BIN" ]]; then
  cargo build -p draconic-cli --release --manifest-path "$ROOT/Cargo.toml"
  if [[ -f "$ROOT/target/release/draconic.exe" ]]; then
    BIN="$ROOT/target/release/draconic.exe"
  else
    BIN="$ROOT/target/release/draconic"
  fi
fi

if [[ ! -f "$BIN" ]]; then
  echo "binary not found: $BIN" >&2
  exit 1
fi

mkdir -p "$OUT"
NAME="draconic-${TRIPLE}"
case "$BIN" in
  *.exe) NAME="draconic-${TRIPLE}.exe" ;;
esac

DEST="$OUT/$NAME"
cp "$BIN" "$DEST"
chmod +x "$DEST"
echo "$DEST"
