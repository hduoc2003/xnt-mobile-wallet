#!/bin/bash

# Script to install the Neptune Wallet APK to an ADB-connected Android device
# Usage: ./install-apk.sh [apk-path]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get the script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APK_DIR="$SCRIPT_DIR/src-tauri/gen/android/app/build/outputs/apk/universal/release"
APK_NAME="app-universal-release-signed.apk"
DEFAULT_APK="$APK_DIR/$APK_NAME"

# Use provided APK path or default
APK_PATH="${1:-$DEFAULT_APK}"

# Check if APK exists
if [ ! -f "$APK_PATH" ]; then
    echo -e "${RED}Error: APK not found at $APK_PATH${NC}"
    echo "Please build the APK first using: npm run tauri android build"
    exit 1
fi

echo -e "${GREEN}Neptune Wallet APK Installer${NC}"
echo "================================"
echo "APK: $APK_PATH"
echo ""

# Check if adb is available
if ! command -v adb &> /dev/null; then
    echo -e "${RED}Error: adb command not found${NC}"
    echo "Please install Android SDK Platform Tools and add it to your PATH"
    exit 1
fi

# Check for connected devices
echo "Checking for connected devices..."
DEVICES=$(adb devices | grep -v "List" | grep "device$" | wc -l | tr -d ' ')

if [ "$DEVICES" -eq 0 ]; then
    echo -e "${RED}Error: No Android devices connected${NC}"
    echo ""
    echo "Please:"
    echo "1. Connect your Android device via USB"
    echo "2. Enable USB debugging on your device"
    echo "3. Accept the USB debugging authorization prompt on your device"
    echo ""
    echo "To check devices, run: adb devices"
    exit 1
fi

# Show connected devices
echo -e "${GREEN}Found $DEVICES connected device(s):${NC}"
adb devices | grep -v "List" | grep "device$" | while read -r line; do
    DEVICE_ID=$(echo "$line" | awk '{print $1}')
    echo "  - $DEVICE_ID"
done
echo ""

# If multiple devices, use the first one (or you can modify to prompt for selection)
if [ "$DEVICES" -gt 1 ]; then
    echo -e "${YELLOW}Warning: Multiple devices connected. Using the first one.${NC}"
    echo "To specify a device, use: adb -s <device-id> install ..."
    echo ""
fi

# Get APK info
APK_SIZE=$(du -h "$APK_PATH" | cut -f1)
echo "APK Size: $APK_SIZE"
echo ""

# Uninstall existing app if present (optional - comment out if you want to keep data)
PACKAGE_NAME="com.draplydo.neptune_wallet_tauri"
echo "Checking for existing installation..."
if adb shell pm list packages | grep -q "$PACKAGE_NAME"; then
    echo -e "${YELLOW}Found existing installation. Uninstalling...${NC}"
    adb uninstall "$PACKAGE_NAME" || echo "Uninstall failed (may not be installed)"
    echo ""
fi

# Check if APK is signed (only if jarsigner is available)
if command -v jarsigner &> /dev/null; then
    echo "Checking APK signature..."
    if ! jarsigner -verify "$APK_PATH" > /dev/null 2>&1; then
        echo -e "${YELLOW}Warning: APK appears to be unsigned${NC}"
        echo ""
        read -p "Sign the APK now? (y/n) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            SIGNED_APK="${APK_PATH%-unsigned.apk}-signed.apk"
            if [ -f "$SIGNED_APK" ]; then
                SIGNED_APK="${APK_PATH%.apk}-signed-$(date +%s).apk"
            fi
            echo "Signing APK..."
            "$SCRIPT_DIR/sign-apk.sh" "$APK_PATH" "$SIGNED_APK"
            APK_PATH="$SIGNED_APK"
            echo ""
        else
            echo -e "${YELLOW}Attempting to install unsigned APK (may fail on some devices)...${NC}"
            echo ""
        fi
    fi
fi

# Install the APK
echo -e "${GREEN}Installing APK...${NC}"
if adb install -r "$APK_PATH"; then
    echo ""
    echo -e "${GREEN}✓ Installation successful!${NC}"
    echo ""
    echo "You can now:"
    echo "1. Launch the app from your device"
    echo "2. View logs using: ./show-logs.sh"
    echo "3. Launch the app using: ./launch-app.sh"
else
    echo ""
    echo -e "${RED}✗ Installation failed${NC}"
    echo ""
    echo "Common issues:"
    echo "- APK not signed (try: ./sign-apk.sh $APK_PATH)"
    echo "- Device storage full"
    echo "- Insufficient permissions"
    echo "- Try: adb install -r -d $APK_PATH"
    echo ""
    echo "To sign the APK first, run:"
    echo "  ./sign-apk.sh $APK_PATH"
    exit 1
fi

