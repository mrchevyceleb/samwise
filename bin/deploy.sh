#!/bin/bash
# Rebuild SamWise with baked-in Doppler secrets, replace /Applications/SamWise.app,
# and restart the launchd-managed instance. Run after any code change.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_SRC="$REPO_DIR/src-tauri/target/release/bundle/macos/SamWise.app"
APP_DST="/Applications/SamWise.app"
AGENT_LABEL="com.mattjohnston.samwise"

cd "$REPO_DIR"

echo "==> Building (doppler run -- npx tauri build)"
doppler run -- npx tauri build

if [ ! -d "$APP_SRC" ]; then
    echo "Build finished but $APP_SRC is missing. Aborting."
    exit 1
fi

echo "==> Stopping launchd-managed SamWise"
launchctl kill SIGTERM "gui/$(id -u)/$AGENT_LABEL" 2>/dev/null || true
pkill -f "SamWise.app/Contents/MacOS/agent-one" 2>/dev/null || true
sleep 2

echo "==> Replacing $APP_DST"
rm -rf "$APP_DST"
cp -R "$APP_SRC" "$APP_DST"

echo "==> Kicking launchd"
launchctl kickstart -k "gui/$(id -u)/$AGENT_LABEL"
sleep 2

echo "==> Running instance:"
pgrep -afl agent-one | grep -v grep | grep SamWise || echo "  (not yet visible, launchd will respawn)"

echo
echo "Done. /Applications/SamWise.app is current."
