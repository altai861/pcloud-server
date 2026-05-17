#!/usr/bin/env bash
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
UNIT_NAME="pcloud-server.service"
USER_UNIT_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
INSTALLED_UNIT="$USER_UNIT_DIR/$UNIT_NAME"

cd "$APP_DIR"

mkdir -p "$USER_UNIT_DIR"

cat > "$INSTALLED_UNIT" <<UNIT
[Unit]
Description=PCloud Rust application server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=$APP_DIR
EnvironmentFile=$APP_DIR/.env
ExecStart=$APP_DIR/scripts/run-prod.sh
Restart=always
RestartSec=3
KillSignal=SIGINT
TimeoutStopSec=30
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=default.target
UNIT

cargo build --release
systemctl --user daemon-reload
systemctl --user enable "$UNIT_NAME" >/dev/null
systemctl --user restart "$UNIT_NAME" 2>/dev/null || systemctl --user start "$UNIT_NAME"
systemctl --user --no-pager --full status "$UNIT_NAME"
