#!/bin/bash

# Script to show logs from Neptune Wallet running on an ADB-connected Android device
# Usage: ./show-logs.sh [filter]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Package name
PACKAGE_NAME="com.draplydo.neptune_wallet_tauri"

# Optional filter (e.g., "Error", "WARN", "neptune", etc.)
FILTER="${1:-}"

echo -e "${GREEN}Neptune Wallet Log Viewer${NC}"
echo "================================"
echo "Package: $PACKAGE_NAME"
if [ -n "$FILTER" ]; then
    echo -e "Filter: ${CYAN}$FILTER${NC}"
fi
echo ""

# Check if adb is available
if ! command -v adb &> /dev/null; then
    echo -e "${RED}Error: adb command not found${NC}"
    echo "Please install Android SDK Platform Tools and add it to your PATH"
    exit 1
fi

# Check for connected devices
DEVICES=$(adb devices | grep -v "List" | grep "device$" | wc -l | tr -d ' ')

if [ "$DEVICES" -eq 0 ]; then
    echo -e "${RED}Error: No Android devices connected${NC}"
    echo ""
    echo "Please connect your Android device and enable USB debugging"
    exit 1
fi

# Check if app is installed
if ! adb shell pm list packages | grep -q "$PACKAGE_NAME"; then
    echo -e "${YELLOW}Warning: App not installed on device${NC}"
    echo "Install it first using: ./install-apk.sh"
    echo ""
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo -e "${BLUE}Starting logcat...${NC}"
echo "Press Ctrl+C to stop"
echo ""
echo "----------------------------------------"
echo ""

# Clear logcat buffer (optional - comment out if you want to see old logs)
# adb logcat -c

# Build logcat command
LOGCAT_CMD="adb logcat"

# Add filter for the package if no custom filter provided
if [ -z "$FILTER" ]; then
    # Show logs from the app and system
    LOGCAT_CMD="$LOGCAT_CMD | grep -E \"($PACKAGE_NAME|AndroidRuntime|FATAL|AndroidRuntime.*FATAL)\""
else
    # Use custom filter
    LOGCAT_CMD="$LOGCAT_CMD | grep -i \"$FILTER\""
fi

# Colorize output
# This will colorize different log levels
LOGCAT_CMD="$LOGCAT_CMD | sed -E 's/(.*E/.*)/\x1b[31m\1\x1b[0m/g; s/(.*W/.*)/\x1b[33m\1\x1b[0m/g; s/(.*I/.*)/\x1b[32m\1\x1b[0m/g; s/(.*D/.*)/\x1b[36m\1\x1b[0m/g'"

# Run logcat with package filter
# Using a more reliable approach with adb logcat filters
if [ -z "$FILTER" ]; then
    # Show all logs but highlight our package
    adb logcat -v time | while IFS= read -r line; do
        if echo "$line" | grep -qi "$PACKAGE_NAME\|AndroidRuntime\|FATAL\|ERROR"; then
            if echo "$line" | grep -qi "ERROR\|FATAL"; then
                echo -e "${RED}$line${NC}"
            elif echo "$line" | grep -qi "WARN"; then
                echo -e "${YELLOW}$line${NC}"
            elif echo "$line" | grep -qi "$PACKAGE_NAME"; then
                echo -e "${GREEN}$line${NC}"
            else
                echo -e "${CYAN}$line${NC}"
            fi
        fi
    done
else
    # Use custom filter
    adb logcat -v time | grep -i "$FILTER" | while IFS= read -r line; do
        if echo "$line" | grep -qi "ERROR\|FATAL"; then
            echo -e "${RED}$line${NC}"
        elif echo "$line" | grep -qi "WARN"; then
            echo -e "${YELLOW}$line${NC}"
        else
            echo "$line"
        fi
    done
fi





