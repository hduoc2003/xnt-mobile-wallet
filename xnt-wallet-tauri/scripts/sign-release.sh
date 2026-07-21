#!/bin/bash
# Sign the release AAB with the upload key and verify it.
# Usage: STOREPASS=... KEYPASS=... scripts/sign-release.sh [path-to-aab]
#
# STOREPASS and KEYPASS are REQUIRED and must come from the environment — never
# hardcode signing passwords in the repo. KEYSTORE and KEYALIAS may also be
# overridden via env (default: <app-root>/upload-keystore.jks, alias "upload").

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)" # the app (xnt-wallet-tauri) root

# Load local signing secrets if present (gitignored; see .env.signing.example).
# shellcheck source=/dev/null
[ -f "$ROOT/.env.signing" ] && . "$ROOT/.env.signing"

AAB_INPUT="${1:-$ROOT/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab}"
KEYSTORE="${KEYSTORE:-$ROOT/upload-keystore.jks}"
STOREPASS="${STOREPASS:?STOREPASS env var is required (the upload keystore password)}"
KEYALIAS="${KEYALIAS:-upload}"
KEYPASS="${KEYPASS:?KEYPASS env var is required (the upload key password)}"

if [[ ! -f "$AAB_INPUT" ]]; then
  echo "Error: AAB not found at $AAB_INPUT"
  echo "Build it with: cd src-tauri/gen/android && ./gradlew bundleRelease"
  exit 1
fi

if [[ ! -f "$KEYSTORE" ]]; then
  echo "Error: Keystore not found at $KEYSTORE"
  exit 1
fi

echo "Signing AAB..."
jarsigner -verbose -sigalg SHA256withRSA -digestalg SHA-256 \
  -keystore "$KEYSTORE" \
  -storepass "$STOREPASS" \
  -keypass "$KEYPASS" \
  "$AAB_INPUT" \
  "$KEYALIAS"

echo "Verifying signature..."
jarsigner -verify -verbose -certs "$AAB_INPUT"

echo "Done. Signed AAB: $AAB_INPUT"









