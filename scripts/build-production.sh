#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Generating API types"
(cd "$ROOT/shared-types" && cargo run --bin generate_api_types)

echo "==> Building GUI (Vite)"
(cd "$ROOT/gui" && npm ci && npm run build)

echo "==> Building dwata-api (release)"
(cd "$ROOT" && cargo build -p dwata-api --release)

echo "==> Done"
echo "Binary: $ROOT/target/release/dwata-api"
