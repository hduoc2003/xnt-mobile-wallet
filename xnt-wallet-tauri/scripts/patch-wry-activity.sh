#!/bin/bash

# Script to patch WryActivity.kt to remove libc++_shared loading (using static C++ linking)
# This needs to be run after each Tauri Android build

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)" # the app (xnt-wallet-tauri) root
WRY_ACTIVITY_FILE="$ROOT/src-tauri/gen/android/app/src/main/java/com/xnt/wallet/generated/WryActivity.kt"

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
    sed -i.sedtmp '/Load C++ runtime library first/,/throw e/d' "$WRY_ACTIVITY_FILE"
    sed -i.sedtmp '/System.loadLibrary("c++_shared")/d' "$WRY_ACTIVITY_FILE"
    
    # Clean up any extra blank lines
    sed -i.sedtmp '/^[[:space:]]*$/N;/^\n$/d' "$WRY_ACTIVITY_FILE"
    # A suffix on -i is the form both GNU and BSD sed accept.
    rm -f "$WRY_ACTIVITY_FILE.sedtmp"
    
    echo "✓ Successfully removed libc++_shared loading code"
else
    echo "WryActivity.kt already patched (no libc++_shared loading)"
fi





