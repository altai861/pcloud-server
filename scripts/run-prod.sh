#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$APP_DIR/target/release/server"

if [[ ! -x "$BINARY" ]]; then
  echo "Release binary not found at $BINARY. Run cargo build --release first." >&2
  exit 1
fi

cd "$APP_DIR"
exec "$BINARY"
