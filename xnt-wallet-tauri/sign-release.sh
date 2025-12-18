#!/bin/bash
# Sign the release AAB with the upload key and verify it.
# Usage: ./sign-release.sh [path-to-aab]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$SCRIPT_DIR"

AAB_INPUT="${1:-$ROOT/src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab}"
KEYSTORE="${KEYSTORE:-$ROOT/upload-keystore.jks}"
STOREPASS="${STOREPASS:-XntUpload#2025}"
KEYALIAS="${KEYALIAS:-upload}"
KEYPASS="${KEYPASS:-XntUpload#2025}"

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




