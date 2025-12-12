#!/bin/bash

# Script to patch WryActivity.kt to remove libc++_shared loading (using static C++ linking)
# This needs to be run after each Tauri Android build

WRY_ACTIVITY_FILE="src-tauri/gen/android/app/src/main/java/com/xnt/wallet/generated/WryActivity.kt"

if [ ! -f "$WRY_ACTIVITY_FILE" ]; then
    echo "Error: WryActivity.kt not found at $WRY_ACTIVITY_FILE"
    exit 1
fi

# Check if libc++_shared loading code exists and remove it
if grep -q "libc++_shared" "$WRY_ACTIVITY_FILE"; then
    echo "Removing libc++_shared loading code (using static linking)"
    # Create backup
    cp "$WRY_ACTIVITY_FILE" "${WRY_ACTIVITY_FILE}.bak"
    
    # Remove the libc++_shared loading code
    sed -i '' '/Load C++ runtime library first/,/throw e/d' "$WRY_ACTIVITY_FILE"
    sed -i '' '/System.loadLibrary("c++_shared")/d' "$WRY_ACTIVITY_FILE"
    
    # Clean up any extra blank lines
    sed -i '' '/^[[:space:]]*$/N;/^\n$/d' "$WRY_ACTIVITY_FILE"
    
    echo "✓ Successfully removed libc++_shared loading code"
else
    echo "WryActivity.kt already patched (no libc++_shared loading)"
fi





