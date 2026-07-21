#!/bin/bash

# Script to run XNT Wallet on iPad Simulator
# Usage: scripts/run-ipad.sh [device_name]

set -e

cd "$(dirname "$0")/.." # run from the app (xnt-wallet-tauri) root

# Default iPad device
DEFAULT_DEVICE="iPad Pro 13-inch (M5)"

# Get device name from argument or use default
DEVICE="${1:-$DEFAULT_DEVICE}"

echo "🚀 Starting XNT Wallet on iPad Simulator..."
echo "📱 Device: $DEVICE"
echo ""

# Check if device exists
if ! xcrun simctl list devices available | grep -q "$DEVICE"; then
    echo "❌ Error: Device '$DEVICE' not found!"
    echo ""
    echo "Available iPad devices:"
    xcrun simctl list devices available | grep -i ipad
    exit 1
fi

# Boot the simulator if not already running
echo "🔧 Booting simulator..."
xcrun simctl boot "$DEVICE" 2>/dev/null || echo "Simulator already booted or running"

# Open Simulator app
open -a Simulator

# Wait a bit for simulator to be ready
echo "⏳ Waiting for simulator to be ready..."
sleep 3

# Run the app
echo "🏃 Running app on $DEVICE..."
npm run tauri:ios:dev -- --verbose "$DEVICE"
