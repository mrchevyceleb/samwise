#!/usr/bin/env bash
# Setup script for the AutoSam LiteLLM proxy
# Run once on the DGX Spark to install and configure the proxy service

set -euo pipefail

echo "=== AutoSam LiteLLM Proxy Setup ==="

# 1. Install LiteLLM with proxy support
if ! command -v litellm &>/dev/null; then
    echo "Installing LiteLLM..."
    python3 -m venv /opt/autosam-litellm-venv 2>/dev/null || python3 -m venv ~/litellm-venv
    VENV=${VENV:-/opt/autosam-litellm-venv}
    $VENV/bin/pip install 'litellm[proxy]'
    echo "LiteLLM installed at $VENV/bin/litellm"
else
    echo "LiteLLM already available"
fi

# 2. Store Fireworks API key securely
KEY_DIR="$HOME/.config/autosam"
mkdir -p "$KEY_DIR"
if [ ! -f "$KEY_DIR/fireworks-api-key" ]; then
    echo -n "Enter your Fireworks API key: "
    read -r FW_KEY
    echo -n "$FW_KEY" > "$KEY_DIR/fireworks-api-key"
    chmod 600 "$KEY_DIR/fireworks-api-key"
    echo "Key stored at $KEY_DIR/fireworks-api-key"
else
    echo "Fireworks API key already stored"
fi

# 3. Install systemd service (user-level)
SERVICE_DIR="$HOME/.config/systemd/user"
mkdir -p "$SERVICE_DIR"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cp "$SCRIPT_DIR/autosam-litellm.service" "$SERVICE_DIR/autosam-litellm.service"
systemctl --user daemon-reload
systemctl --user enable autosam-litellm.service
systemctl --user start autosam-litellm.service

echo ""
echo "=== Proxy Status ==="
sleep 2
systemctl --user status autosam-litellm.service --no-pager || true
echo ""
echo "Proxy URL: http://127.0.0.1:9876"
echo "Health check: curl http://127.0.0.1:9876/health"
echo ""
echo "In AutoSam Settings, set:"
echo "  LLM Proxy Enabled: true"
echo "  LLM Proxy Base URL: http://127.0.0.1:9876"
echo "  LLM Proxy API Key: (anything, LiteLLM doesn't enforce it)"
echo "  LLM Proxy Backend: fireworks-glm-5.1"
