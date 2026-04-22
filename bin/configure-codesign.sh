#!/bin/bash
# Wire the self-signed cert into tauri.conf.json under bundle.macOS.signingIdentity
# so every `tauri build` uses the same identity. TCC grants then persist across
# rebuilds because macOS binds them to the signing identity, not the binary hash.

set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CONF="$REPO_DIR/src-tauri/tauri.conf.json"
IDENTITY="SamWise Self-Signed"

if ! security find-certificate -c "$IDENTITY" >/dev/null 2>&1; then
    echo "Cert \"$IDENTITY\" not found in keychain. Run bin/setup-codesign.sh first."
    exit 1
fi

# Use Python because jq isn't guaranteed, and json.tool preserves structure.
python3 - <<PY
import json, sys
path = "$CONF"
with open(path) as f:
    conf = json.load(f)

bundle = conf.setdefault("bundle", {})
mac = bundle.setdefault("macOS", {})
mac["signingIdentity"] = "$IDENTITY"
# providerShortName is only needed for notarization; leave it blank.

with open(path, "w") as f:
    json.dump(conf, f, indent=2)
    f.write("\n")

print(f"Set bundle.macOS.signingIdentity = {mac['signingIdentity']}")
PY

echo "Done. Next build will sign with \"$IDENTITY\"."
