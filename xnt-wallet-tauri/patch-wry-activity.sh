#!/bin/bash

# Script to patch WryActivity.kt to load libc++_shared before the main library
# This needs to be run after each Tauri Android build

WRY_ACTIVITY_FILE="src-tauri/gen/android/app/src/main/java/com/xnt/wallet/generated/WryActivity.kt"

if [ ! -f "$WRY_ACTIVITY_FILE" ]; then
    echo "Error: WryActivity.kt not found at $WRY_ACTIVITY_FILE"
    exit 1
fi

# Check if already patched
if grep -q "libc++_shared" "$WRY_ACTIVITY_FILE"; then
    echo "WryActivity.kt already patched"
    exit 0
fi

# Create backup
cp "$WRY_ACTIVITY_FILE" "${WRY_ACTIVITY_FILE}.bak"

# Patch the file
sed -i '' '/companion object {/,/}/ {
    /System.loadLibrary("neptune_wallet_tauri_lib")/i\
            // Load C++ runtime library first (required for native library)\
            try {\
                System.loadLibrary("c++_shared")\
            } catch (e: UnsatisfiedLinkError) {\
                android.util.Log.e("WryActivity", "CRITICAL: Failed to load libc++_shared.so: ${e.message}")\
                throw e\
            }\
            // Load the main native library
}' "$WRY_ACTIVITY_FILE"

if [ $? -eq 0 ]; then
    echo "✓ Successfully patched WryActivity.kt"
else
    echo "✗ Failed to patch WryActivity.kt"
    mv "${WRY_ACTIVITY_FILE}.bak" "$WRY_ACTIVITY_FILE"
    exit 1
fi





