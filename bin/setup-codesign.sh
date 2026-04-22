#!/bin/bash
# Create a self-signed code-signing certificate in the login keychain,
# trust it for codesign, and print the identity name so tauri.conf.json
# can use it. Run once; resulting cert lives forever.

set -euo pipefail

CERT_NAME="SamWise Self-Signed"
CERT_DIR="$HOME/.samwise-codesign"
CERT_KEY="$CERT_DIR/sign.key"
CERT_CSR="$CERT_DIR/sign.csr"
CERT_CRT="$CERT_DIR/sign.crt"
CERT_CONF="$CERT_DIR/sign.conf"

mkdir -p "$CERT_DIR"

# Already installed?
if security find-certificate -c "$CERT_NAME" >/dev/null 2>&1; then
    echo "Cert \"$CERT_NAME\" already in keychain. Nothing to do."
    echo "Signing identity to use in tauri.conf.json: $CERT_NAME"
    exit 0
fi

# OpenSSL config for a proper code-signing cert (needs the codeSigning extended
# key usage so `codesign` will accept it).
cat > "$CERT_CONF" <<EOF
[req]
distinguished_name = dn
prompt = no
req_extensions = req_ext
x509_extensions = req_ext

[dn]
CN = $CERT_NAME
O  = SamWise
OU = Personal
C  = US

[req_ext]
basicConstraints = CA:FALSE
keyUsage = digitalSignature
extendedKeyUsage = codeSigning
EOF

echo "==> Generating key + self-signed cert"
openssl genrsa -out "$CERT_KEY" 2048 2>/dev/null
openssl req -new -key "$CERT_KEY" -out "$CERT_CSR" -config "$CERT_CONF" 2>/dev/null
openssl x509 -req -in "$CERT_CSR" -signkey "$CERT_KEY" -out "$CERT_CRT" \
    -days 3650 -extfile "$CERT_CONF" -extensions req_ext -sha256 2>/dev/null

# Bundle into a .p12 for keychain import.
# macOS `security import` uses an older PKCS12 MAC than OpenSSL 3 defaults,
# so force -legacy + an explicit password (blank breaks macOS on some versions).
P12_PASS="samwise"
openssl pkcs12 -export -legacy -inkey "$CERT_KEY" -in "$CERT_CRT" \
    -out "$CERT_DIR/sign.p12" -password "pass:$P12_PASS" -name "$CERT_NAME"

echo "==> Importing into login keychain (may prompt for Mac login password)"
security import "$CERT_DIR/sign.p12" -k ~/Library/Keychains/login.keychain-db \
    -P "$P12_PASS" -T /usr/bin/codesign -T /usr/bin/security -A

echo "==> Marking cert as trusted for codesigning (may prompt again)"
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain \
    -p codeSign "$CERT_CRT"

echo
echo "Done."
echo "Signing identity: $CERT_NAME"
echo "Next: ./bin/configure-codesign.sh to wire it into tauri.conf.json"
