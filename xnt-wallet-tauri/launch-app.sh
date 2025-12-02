#!/bin/bash

# Script to launch Neptune Wallet on an ADB-connected Android device
# Usage: ./launch-app.sh

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PACKAGE_NAME="com.draplydo.neptune_wallet_tauri"
MAIN_ACTIVITY="$PACKAGE_NAME/.MainActivity"

echo -e "${GREEN}Launching Neptune Wallet...${NC}"

# Check if adb is available
if ! command -v adb &> /dev/null; then
    echo -e "${RED}Error: adb command not found${NC}"
    exit 1
fi

# Check for connected devices
DEVICES=$(adb devices | grep -v "List" | grep "device$" | wc -l | tr -d ' ')

if [ "$DEVICES" -eq 0 ]; then
    echo -e "${RED}Error: No Android devices connected${NC}"
    exit 1
fi

# Check if app is installed
if ! adb shell pm list packages | grep -q "$PACKAGE_NAME"; then
    echo -e "${RED}Error: App not installed${NC}"
    echo "Install it first using: ./install-apk.sh"
    exit 1
fi

# Launch the app
echo "Starting $MAIN_ACTIVITY..."
if adb shell am start -n "$MAIN_ACTIVITY"; then
    echo -e "${GREEN}✓ App launched successfully!${NC}"
    echo ""
    echo "View logs with: ./show-logs.sh"
else
    echo -e "${RED}✗ Failed to launch app${NC}"
    exit 1
fi

