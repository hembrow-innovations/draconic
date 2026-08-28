#!/usr/bin/env bash
# D01.02: download the host-triple CLI artifact and place `draconic` on PATH.
set -euo pipefail

usage() {
  echo "Usage: scripts/install.sh [--from URL_OR_DIR_OR_FILE] [--dir DIR] [--triple TRIPLE]" >&2
  echo "One-line: curl -fsSL https://raw.githubusercontent.com/hembrow-innovations/draconic/main/scripts/install.sh | sh" >&2
}

FROM="${DRACONIC_INSTALL_FROM:-}"
DIR="${DRACONIC_INSTALL_DIR:-}"
TRIPLE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from)
      FROM="${2:-}"
      shift 2
      ;;
    --dir)
      DIR="${2:-}"
      shift 2
      ;;
    --triple)
      TRIPLE="${2:-}"
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

DIR="${DIR:-$HOME/.draconic/bin}"

host_triple_rustc() {
  command -v rustc >/dev/null 2>&1 || return 1
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

host_triple_uname() {
  local sys mach
  sys="$(uname -s)"
  mach="$(uname -m)"
  case "$sys" in
    Darwin)
      case "$mach" in
        arm64) printf '%s\n' "aarch64-apple-darwin" ;;
        x86_64) printf '%s\n' "x86_64-apple-darwin" ;;
        *)
          echo "unsupported host: $sys $mach" >&2
          return 1
          ;;
      esac
      ;;
    Linux)
      case "$mach" in
        x86_64) printf '%s\n' "x86_64-unknown-linux-gnu" ;;
        aarch64 | arm64) printf '%s\n' "aarch64-unknown-linux-gnu" ;;
        *)
          echo "unsupported host: $sys $mach" >&2
          return 1
          ;;
      esac
      ;;
    *)
      echo "unsupported host: $sys $mach" >&2
      return 1
      ;;
  esac
}

if [[ -z "$TRIPLE" ]]; then
  TRIPLE="$(host_triple_rustc || host_triple_uname)"
fi
if [[ -z "$TRIPLE" ]]; then
  echo "could not detect host triple" >&2
  exit 1
fi

ARTIFACT="draconic-${TRIPLE}"
INSTALLED="draconic"
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    ARTIFACT="draconic-${TRIPLE}.exe"
    INSTALLED="draconic.exe"
    ;;
esac

DEFAULT_URL="https://github.com/hembrow-innovations/draconic/releases/latest/download/${ARTIFACT}"
if [[ -z "$FROM" ]]; then
  FROM="$DEFAULT_URL"
fi

mkdir -p "$DIR"
DEST="$DIR/$INSTALLED"
TMP="$(mktemp)"
cleanup() {
  rm -f "$TMP"
}
trap cleanup EXIT

fetch() {
  local src="$1"
  if [[ "$src" == http://* || "$src" == https://* ]]; then
    if ! command -v curl >/dev/null 2>&1; then
      echo "curl is required to download $src" >&2
      exit 1
    fi
    curl -fsSL "$src" -o "$TMP"
    return
  fi
  if [[ -d "$src" ]]; then
    if [[ ! -f "$src/$ARTIFACT" ]]; then
      echo "artifact not found: $src/$ARTIFACT" >&2
      exit 1
    fi
    cp "$src/$ARTIFACT" "$TMP"
    return
  fi
  if [[ -f "$src" ]]; then
    cp "$src" "$TMP"
    return
  fi
  echo "not found: $src" >&2
  exit 1
}

fetch "$FROM"
cp "$TMP" "$DEST"
chmod +x "$DEST"

echo "$DEST"
case ":$PATH:" in
  *":$DIR:"*) ;;
  *)
    echo "Add to PATH: export PATH=\"$DIR:\$PATH\"" >&2
    ;;
esac
