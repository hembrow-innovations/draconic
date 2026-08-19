#!/usr/bin/env bash
# Build the Draconic todo client (JS) and pure Draconic native static HTTP host.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
DRACONIC="${DRACONIC:-$REPO/target/debug/draconic}"
PORT="${PORT:-18083}"
OUT="${OUT:-$ROOT/server-bin}"

if [[ ! -x "$DRACONIC" ]]; then
  echo "building draconic CLI…"
  cargo build -q -p draconic-cli --manifest-path "$REPO/Cargo.toml"
  DRACONIC="$REPO/target/debug/draconic"
fi

echo "compiling src/todo.drac → public/todo.js"
"$DRACONIC" build --target js "$ROOT/src/todo.drac" -o "$ROOT/public/todo.js"

echo "compiling server.drac → native static host"
"$DRACONIC" build --target native "$ROOT/server.drac" -o "$OUT"

echo
echo "done."
echo "  run:  (cd $ROOT && $OUT)"
echo "  open: http://127.0.0.1:$PORT/"
