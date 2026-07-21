#!/bin/bash

# Script to sign the Neptune Wallet APK with a debug keystore
# Uses apksigner (preferred) or falls back to jarsigner
# Usage: scripts/sign-apk.sh [apk-path] [output-path]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)" # the app (xnt-wallet-tauri) root
APK_DIR="$ROOT/src-tauri/gen/android/app/build/outputs/apk/universal/release"
UNSIGNED_APK="$APK_DIR/app-universal-release-unsigned.apk"
DEFAULT_APK="$APK_DIR/app-universal-release.apk"
KEYSTORE_DIR="$ROOT"
KEYSTORE_FILE="$KEYSTORE_DIR/debug.keystore"
KEYSTORE_PASSWORD="android"
KEY_ALIAS="androiddebugkey"
KEY_PASSWORD="android"

# Find apksigner (preferred for Android)
APKSIGNER=""
if command -v apksigner &> /dev/null; then
    APKSIGNER="apksigner"
elif [ -f "/Users/draplydo/Library/Android/sdk/build-tools/35.0.0/apksigner" ]; then
    APKSIGNER="/Users/draplydo/Library/Android/sdk/build-tools/35.0.0/apksigner"
else
    # Try to find it in common locations
    for build_tools in /Users/draplydo/Library/Android/sdk/build-tools/*/apksigner; do
        if [ -f "$build_tools" ]; then
            APKSIGNER="$build_tools"
            break
        fi
    done
fi

USE_APKSIGNER=false
if [ -n "$APKSIGNER" ]; then
    USE_APKSIGNER=true
    echo -e "${GREEN}Using apksigner (recommended for Android)${NC}"
else
    echo -e "${YELLOW}apksigner not found, will use jarsigner${NC}"
fi

# Use provided APK path or default (fallback to actual Gradle output name)
if [ -n "$1" ]; then
    INPUT_APK="$1"
elif [ -f "$UNSIGNED_APK" ]; then
    INPUT_APK="$UNSIGNED_APK"
else
    INPUT_APK="$DEFAULT_APK"
fi

# Output path
if [ -n "$2" ]; then
    OUTPUT_APK="$2"
else
    # Default: same directory, with -signed suffix
    OUTPUT_DIR=$(dirname "$INPUT_APK")
    INPUT_NAME=$(basename "$INPUT_APK" .apk)
    OUTPUT_APK="$OUTPUT_DIR/${INPUT_NAME%-unsigned}-signed.apk"
fi

echo -e "${GREEN}Neptune Wallet APK Signer${NC}"
echo "================================"
echo "Input APK:  $INPUT_APK"
echo "Output APK: $OUTPUT_APK"
echo ""

# Check if input APK exists
if [ ! -f "$INPUT_APK" ]; then
    echo -e "${RED}Error: APK not found at $INPUT_APK${NC}"
    echo "Please build the APK first using: npm run tauri android build"
    exit 1
fi

# Check if jarsigner is available
if ! command -v jarsigner &> /dev/null; then
    echo -e "${RED}Error: jarsigner command not found${NC}"
    echo "Please install Java JDK and add it to your PATH"
    exit 1
fi

# Create debug keystore if it doesn't exist
if [ ! -f "$KEYSTORE_FILE" ]; then
    echo -e "${YELLOW}Creating debug keystore...${NC}"
    keytool -genkey -v \
        -keystore "$KEYSTORE_FILE" \
        -alias "$KEY_ALIAS" \
        -keyalg RSA \
        -keysize 2048 \
        -validity 10000 \
        -storepass "$KEYSTORE_PASSWORD" \
        -keypass "$KEY_PASSWORD" \
        -dname "CN=Android Debug, O=Android, C=US"
    echo -e "${GREEN}✓ Debug keystore created${NC}"
    echo ""
fi

# Remove existing signed APK if it exists
if [ -f "$OUTPUT_APK" ]; then
    echo -e "${YELLOW}Removing existing signed APK...${NC}"
    rm "$OUTPUT_APK"
fi

# Copy unsigned APK to output location
echo "Copying APK..."
cp "$INPUT_APK" "$OUTPUT_APK"

# Align the APK FIRST (before signing)
if command -v zipalign &> /dev/null; then
    echo "Aligning APK (before signing)..."
    ALIGNED_APK="${OUTPUT_APK%.apk}-aligned-temp.apk"
    zipalign -v -f 4 "$OUTPUT_APK" "$ALIGNED_APK"
    if [ $? -eq 0 ]; then
        mv "$ALIGNED_APK" "$OUTPUT_APK"
        echo -e "${GREEN}✓ APK aligned${NC}"
    fi
else
    echo -e "${YELLOW}Note: zipalign not found, skipping alignment${NC}"
fi

# Sign the APK
if [ "$USE_APKSIGNER" = true ]; then
    # Use apksigner (preferred for Android)
    echo -e "${GREEN}Signing APK with apksigner...${NC}"
    "$APKSIGNER" sign \
        --ks "$KEYSTORE_FILE" \
        --ks-pass "pass:$KEYSTORE_PASSWORD" \
        --ks-key-alias "$KEY_ALIAS" \
        --key-pass "pass:$KEY_PASSWORD" \
        --out "$OUTPUT_APK" \
        "$OUTPUT_APK"
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ APK signed successfully${NC}"
    else
        echo -e "${RED}✗ Signing failed${NC}"
        exit 1
    fi
    
    # Verify with apksigner
    echo ""
    echo "Verifying signature..."
    "$APKSIGNER" verify --verbose "$OUTPUT_APK"
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ APK verified successfully${NC}"
    else
        echo -e "${YELLOW}Warning: Verification had issues${NC}"
    fi
else
    # Fall back to jarsigner
    echo -e "${GREEN}Signing APK with jarsigner...${NC}"
    jarsigner -verbose \
        -sigalg SHA256withRSA \
        -digestalg SHA-256 \
        -keystore "$KEYSTORE_FILE" \
        -storepass "$KEYSTORE_PASSWORD" \
        -keypass "$KEY_PASSWORD" \
        "$OUTPUT_APK" \
        "$KEY_ALIAS"
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ APK signed successfully${NC}"
    else
        echo -e "${RED}✗ Signing failed${NC}"
        exit 1
    fi
    
    # Align again after signing (required for jarsigner)
    if command -v zipalign &> /dev/null; then
        echo ""
        echo "Re-aligning APK (after signing)..."
        ALIGNED_APK="${OUTPUT_APK%.apk}-aligned-final.apk"
        zipalign -v -f 4 "$OUTPUT_APK" "$ALIGNED_APK"
        if [ $? -eq 0 ]; then
            mv "$ALIGNED_APK" "$OUTPUT_APK"
            echo -e "${GREEN}✓ APK re-aligned${NC}"
        fi
    fi
    
    # Verify the signature
    echo ""
    echo "Verifying signature..."
    jarsigner -verify -verbose -certs "$OUTPUT_APK" > /dev/null 2>&1
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ Signature verified${NC}"
    else
        echo -e "${YELLOW}Warning: Signature verification had issues${NC}"
    fi
fi

echo ""
echo -e "${GREEN}✓ Signed APK ready: $OUTPUT_APK${NC}"
echo ""
echo "You can now install it using:"
echo "  scripts/install-apk.sh $OUTPUT_APK"
echo "  or"
echo "  adb install $OUTPUT_APK"
