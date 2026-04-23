#!/bin/bash
# Rebuild SamWise with baked-in Doppler secrets, replace /Applications/SamWise.app,
# and restart the launchd-managed instance. Run after any code change.
#
# Safety: refuses to kill the running app if Sam has a task in flight or a
# Codex review is active. Pass --force to override (kills children anyway).
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
APP_SRC="$REPO_DIR/src-tauri/target/release/bundle/macos/SamWise.app"
APP_DST="/Applications/SamWise.app"
AGENT_LABEL="com.mattjohnston.samwise"

FORCE=0
for arg in "$@"; do
    case "$arg" in
        --force) FORCE=1 ;;
    esac
done

cd "$REPO_DIR"

# ── In-flight guard ─────────────────────────────────────────────────
# Restarting Samwise kills any Claude Code / Codex child processes it
# spawned. That's destructive when a task or review is mid-run.
check_inflight() {
    local SB_URL SB_KEY
    SB_URL="$(doppler secrets get SB_URL --plain --project agent-one --config prd 2>/dev/null || true)"
    SB_KEY="$(doppler secrets get SB_SERVICE_ROLE_KEY --plain --project agent-one --config prd 2>/dev/null || true)"
    if [ -z "$SB_URL" ] || [ -z "$SB_KEY" ]; then
        echo "  (couldn't load Supabase creds from Doppler, skipping in-flight check)"
        return 0
    fi

    # Tasks claimed and still running, OR cards in review whose codex run is
    # recent (<12 min). The codex review timeout is 10 minutes, so the window
    # must be wider than that or we'd clear the guard while a review is still
    # active between minutes 5-10 and kill its child process.
    local RESP
    RESP=$(curl -sS -G \
        -H "apikey: $SB_KEY" \
        -H "Authorization: Bearer $SB_KEY" \
        --data-urlencode "select=id,title,status,last_pr_review_at" \
        --data-urlencode "or=(status.eq.in_progress,and(status.eq.review,last_pr_review_at.gt.$(date -u -v-12M +%Y-%m-%dT%H:%M:%SZ)))" \
        "$SB_URL/rest/v1/ae_tasks" || echo "[]")

    if [ "$RESP" = "[]" ] || [ -z "$RESP" ]; then
        return 0
    fi

    echo "!! Sam has work in flight:"
    echo "$RESP" | python3 -c "import sys,json; [print(f'   [{t[\"status\"]}] {t[\"title\"]}') for t in json.load(sys.stdin)]" 2>/dev/null || echo "$RESP"
    echo
    echo "Restarting now would kill the Claude Code / Codex child processes."
    echo "Either wait, or pass --force to override."
    exit 1
}

if [ "$FORCE" -eq 0 ]; then
    echo "==> Checking for in-flight work"
    check_inflight
else
    echo "==> --force given, skipping in-flight check"
fi

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
