#!/bin/bash

# Script to launch XNT Wallet app on iPad Simulator
# This will install and launch the app if it's already built

set -e

cd "$(dirname "$0")/.." # run from the app (xnt-wallet-tauri) root

BUNDLE_ID="com.xnt.wallet"
DEVICE_ID="F55431B4-90F5-4B95-AC36-9DBDD6392A9D"  # iPad Pro 13-inch (M5)

echo "🔍 Checking if app is installed..."

# Find the .app bundle in multiple possible locations
APP_BUNDLE=$(find src-tauri/gen/apple/build -name "XNT Wallet.app" -type d 2>/dev/null | head -1)

if [ -z "$APP_BUNDLE" ]; then
    APP_BUNDLE=$(find src-tauri/target -name "XNT Wallet.app" -type d 2>/dev/null | head -1)
fi

if [ -z "$APP_BUNDLE" ]; then
    echo "❌ App bundle not found. Please build the app first:"
    echo "   npm run tauri:ios:dev:ipad"
    echo ""
    echo "💡 Or open Xcode and build from there:"
    echo "   open src-tauri/gen/apple/xnt-wallet-tauri.xcodeproj"
    exit 1
fi

echo "✅ Found app bundle: $APP_BUNDLE"

# Install the app
echo "📦 Installing app to simulator..."
xcrun simctl install "$DEVICE_ID" "$APP_BUNDLE"

if [ $? -eq 0 ]; then
    echo "✅ App installed successfully"
else
    echo "❌ Failed to install app"
    exit 1
fi

# Launch the app
echo "🚀 Launching app..."
xcrun simctl launch "$DEVICE_ID" "$BUNDLE_ID"

if [ $? -eq 0 ]; then
    echo "✅ App launched successfully!"
    echo ""
    echo "💡 Tip: If you don't see the app, check the simulator home screen"
else
    echo "❌ Failed to launch app"
    exit 1
fi
