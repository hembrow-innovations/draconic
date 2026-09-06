#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
DRACONIC="${DRACONIC:-$REPO/target/debug/draconic}"

if [[ ! -x "$DRACONIC" ]]; then
  echo "building draconic CLI…"
  cargo build -q -p draconic-cli --manifest-path "$REPO/Cargo.toml"
  DRACONIC="$REPO/target/debug/draconic"
fi

exec "$DRACONIC" run --target js "$ROOT/types.drac"
