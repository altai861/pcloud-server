#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIT_NAME="pcloud-server.service"
USER_UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
INSTALLED_UNIT="$USER_UNIT_DIR/$UNIT_NAME"
SOURCE_UNIT="$APP_DIR/systemd/user/$UNIT_NAME"

cd "$APP_DIR"

mkdir -p "$USER_UNIT_DIR"

if [[ ! -f "$INSTALLED_UNIT" ]]; then
  cp "$SOURCE_UNIT" "$INSTALLED_UNIT"
fi

cargo build --release
systemctl --user daemon-reload
systemctl --user enable "$UNIT_NAME" >/dev/null
systemctl --user restart "$UNIT_NAME" 2>/dev/null || systemctl --user start "$UNIT_NAME"
systemctl --user --no-pager --full status "$UNIT_NAME"
