#!/usr/bin/env bash
# Build the Draconic todo client (JS) and native static HTTP host.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$ROOT/../.." && pwd)"
DRACONIC="${DRACONIC:-$REPO/target/debug/draconic}"
PORT="${PORT:-8080}"

if [[ ! -x "$DRACONIC" ]]; then
  echo "building draconic CLI…"
  cargo build -q -p draconic-cli --manifest-path "$REPO/Cargo.toml"
  DRACONIC="$REPO/target/debug/draconic"
fi

echo "compiling src/todo.drac → public/todo.js"
"$DRACONIC" build --target js "$ROOT/src/todo.drac" -o "$ROOT/public/todo.js"

echo "building native HTTP server"
make -C "$ROOT/server" -s

echo
echo "done."
echo "  run:  $ROOT/server/server $PORT $ROOT/public"
echo "  open: http://127.0.0.1:$PORT/"
