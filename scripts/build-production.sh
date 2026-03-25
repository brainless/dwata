#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Tauri packaging is macOS-only for now. Run this script on macOS."
  exit 1
fi

echo "==> Generating API types"
(cd "$ROOT/shared-types" && cargo run --bin generate_api_types)

echo "==> Installing GUI dependencies"
(cd "$ROOT/gui" && npm ci)

echo "==> Building dwata-api (release)"
CONFIG_PATH="${DWATA_OAUTH_CONFIG_PATH:-}"
if [ -z "$CONFIG_PATH" ]; then
  case "$(uname -s)" in
    Darwin) CONFIG_PATH="$HOME/Library/Application Support/dwata/api.toml" ;;
    Linux) CONFIG_PATH="$HOME/.config/dwata/api.toml" ;;
    MINGW*|MSYS*|CYGWIN*) CONFIG_PATH="${APPDATA:-}/dwata/api.toml" ;;
  esac
fi

if [ -n "${CONFIG_PATH}" ] && [ -f "${CONFIG_PATH}" ] && command -v python3 >/dev/null 2>&1; then
  export CONFIG_PATH
  OAUTH_VALUES="$(python3 - <<'PY'
import os
import sys

config_path = os.environ.get("CONFIG_PATH")
if not config_path:
    sys.exit(0)

try:
    import tomllib
except Exception:
    sys.exit(0)

try:
    with open(config_path, "rb") as f:
        data = tomllib.load(f)
except Exception:
    sys.exit(0)

google = data.get("google_oauth") or {}
client_id = str(google.get("client_id") or "")
client_secret = str(google.get("client_secret") or "")
print(client_id)
print(client_secret)
PY
)"
  CLIENT_ID="$(printf '%s' "$OAUTH_VALUES" | sed -n '1p')"
  CLIENT_SECRET="$(printf '%s' "$OAUTH_VALUES" | sed -n '2p')"
  if [ -n "$CLIENT_ID" ]; then
    export DWATA_DEFAULT_GOOGLE_CLIENT_ID="$CLIENT_ID"
  fi
  if [ -n "$CLIENT_SECRET" ]; then
    export DWATA_DEFAULT_GOOGLE_CLIENT_SECRET="$CLIENT_SECRET"
  fi
fi

(cd "$ROOT" && CONFIG_PATH="$CONFIG_PATH" cargo build -p dwata-api --release)

TARGET_TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "$TARGET_TRIPLE" ]]; then
  echo "Failed to determine Rust host target triple."
  exit 1
fi

SIDECAR_DIR="$ROOT/tauri/bin"
SIDECAR_BIN="$SIDECAR_DIR/dwata-api-$TARGET_TRIPLE"
mkdir -p "$SIDECAR_DIR"
cp "$ROOT/target/release/dwata-api" "$SIDECAR_BIN"
chmod +x "$SIDECAR_BIN"

echo "==> Building Tauri app (macOS)"
(cd "$ROOT/tauri" && npm ci && npm run build)

echo "==> Done"
echo "Tauri bundle output: $ROOT/tauri/src-tauri/target/release/bundle"
