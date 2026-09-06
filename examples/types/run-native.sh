#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
DRACONIC="${DRACONIC:-$REPO/target/debug/draconic}"
OUT="${OUT:-$ROOT/types-bin}"

if [[ ! -x "$DRACONIC" ]]; then
  echo "building draconic CLI…"
  cargo build -q -p draconic-cli --manifest-path "$REPO/Cargo.toml"
  DRACONIC="$REPO/target/debug/draconic"
fi

"$DRACONIC" build --target native "$ROOT/types-native.drac" -o "$OUT"
exec "$OUT"
