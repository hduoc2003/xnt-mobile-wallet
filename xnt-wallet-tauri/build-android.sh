#!/bin/bash
# Build script for Android 64-bit only (arm64-v8a and x86_64)

set -e

# Set Android NDK path
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/Library/Android/sdk/ndk/26.1.10909125}"
export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"

# Verify NDK exists
if [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "Error: Android NDK not found at $ANDROID_NDK_HOME"
    echo "Please set ANDROID_NDK_HOME environment variable or install NDK"
    exit 1
fi

# Add NDK toolchain to PATH
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"

# Set CMake toolchain file
export CMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_HOME/build/cmake/android.toolchain.cmake"

# Default Android build toolchain variables (can be overridden by caller)
export ANDROID_ABI="${ANDROID_ABI:-arm64-v8a}"
export ANDROID_PLATFORM="${ANDROID_PLATFORM:-android-24}"
export CC="${CC:-aarch64-linux-android24-clang}"
export CXX="${CXX:-aarch64-linux-android24-clang++}"
export AR="${AR:-llvm-ar}"

# Verify CMake toolchain exists
if [ ! -f "$CMAKE_TOOLCHAIN_FILE" ]; then
    echo "Error: CMake toolchain file not found at $CMAKE_TOOLCHAIN_FILE"
    exit 1
fi

echo "Building for Android 64-bit architectures (arm64-v8a, x86_64)..."
echo "NDK: $ANDROID_NDK_HOME"
echo "CMake Toolchain: $CMAKE_TOOLCHAIN_FILE"

cd "$(dirname "$0")"

echo ""
echo "Step 1: Building web frontend (Vite)..."
npm run build

echo ""
echo "Step 2: Building Tauri Android targets..."
echo "  - arm64-v8a (aarch64)"
npm run tauri android build -- --target aarch64
echo "  - x86_64"
npm run tauri android build -- --target x86_64

echo ""
echo "Step 3: Patching WryActivity for libc++ load order..."
./patch-wry-activity.sh

echo ""
echo "Step 4: Assembling universal release APK via Gradle..."
cd src-tauri/gen/android
./gradlew assembleRelease -x rustBuildArm64Release -x rustBuildX86_64Release
cd ../../..

UNSIGNED_APK="src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk"
SIGNED_APK="src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-signed.apk"

if [ ! -f "$UNSIGNED_APK" ]; then
  echo "Error: unsigned APK not found at $UNSIGNED_APK"
  exit 1
fi

echo ""
echo "Step 5: Signing universal release APK..."
./sign-apk.sh "$UNSIGNED_APK" "$SIGNED_APK"

echo ""
echo "Build + patch + sign complete!"
echo "  Signed APK: $SIGNED_APK"
echo ""
echo "You can install it with:"
echo "  ./install-apk.sh \"$SIGNED_APK\""


