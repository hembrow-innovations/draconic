#!/usr/bin/env bash
# Start static build from the sibling draconic-web repo, stage HTML to dist/pages for GitHub Pages.
set -euo pipefail

usage() {
  echo "Usage: scripts/generate-website.sh [--bin PATH] [--out DIR]" >&2
}

OUT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin)
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

if [[ -n "${DRACONIC_WEB:-}" ]]; then
  SITE="$DRACONIC_WEB"
else
  SITE="$ROOT/../draconic-web"
fi
if [[ ! -d "$SITE" ]]; then
  echo "public site not found at $SITE (set DRACONIC_WEB)" >&2
  exit 1
fi
SITE="$(cd "$SITE" && pwd)"

PAGES_BASE="${PAGES_BASE:-/draconic}" pnpm --dir "$SITE" build

CLIENT=""
for candidate in "$SITE/dist/client" "$SITE/dist"; do
  if [[ -f "$candidate/index.html" ]]; then
    CLIENT="$candidate"
    break
  fi
done
if [[ -z "$CLIENT" ]]; then
  echo "Start static build produced no index.html under $SITE/dist" >&2
  exit 1
fi

mkdir -p "$OUT"
cp -R "$CLIENT"/. "$OUT"/
touch "$OUT/.nojekyll"
echo "$OUT"
