fn main() {
    tauri_build::build();
    
    // Configure Android NDK environment for CMake builds
    if let Ok(target) = std::env::var("TARGET") {
        if target.contains("android") {
            // Set Android NDK environment variables for CMake
            if let Ok(ndk_home) = std::env::var("ANDROID_NDK_HOME") {
                println!("cargo:rustc-env=ANDROID_NDK_HOME={}", ndk_home);
                println!("cargo:rustc-env=ANDROID_NDK_ROOT={}", ndk_home);
                println!("cargo:rustc-link-lib=c++_shared");
                
                // Set CMake toolchain file
                let toolchain_file = format!("{}/build/cmake/android.toolchain.cmake", ndk_home);
                if std::path::Path::new(&toolchain_file).exists() {
                    println!("cargo:rustc-env=CMAKE_TOOLCHAIN_FILE={}", toolchain_file);
                }
                
                // Configure CMake for Android builds
                println!("cargo:rustc-env=CMAKE_SYSTEM_NAME=Android");
                println!("cargo:rustc-env=CMAKE_ANDROID_NDK={}", ndk_home);
                
                if target.contains("aarch64") {
                    println!("cargo:rustc-env=CMAKE_ANDROID_ARCH_ABI=arm64-v8a");
                    println!("cargo:rustc-env=CMAKE_SYSTEM_PROCESSOR=aarch64");
                } else if target.contains("x86_64") {
                    println!("cargo:rustc-env=CMAKE_ANDROID_ARCH_ABI=x86_64");
                    println!("cargo:rustc-env=CMAKE_SYSTEM_PROCESSOR=x86_64");
                }
                
                // Configure CMake to build static libraries and set proper paths
                println!("cargo:rustc-env=CMAKE_BUILD_TYPE=Release");
                println!("cargo:rustc-env=CMAKE_POSITION_INDEPENDENT_CODE=ON");
                println!("cargo:rustc-env=BUILD_SHARED_LIBS=OFF");
                println!("cargo:rustc-env=CMAKE_POLICY_VERSION_MINIMUM=3.9");
                
                // Help CMake find libraries (for leveldb-sys snappy linking)
                // The build directory will contain the snappy library after it's built
                let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
                if !out_dir.is_empty() {
                    // Set paths where CMake will look for libraries
                    println!("cargo:rustc-env=CMAKE_PREFIX_PATH={}", out_dir);
                    println!("cargo:rustc-env=CMAKE_LIBRARY_PATH={}", out_dir);
                    println!("cargo:rustc-env=CMAKE_FIND_ROOT_PATH={}", out_dir);
                    
                    // For leveldb-sys, we need to tell it where to find snappy
                    // The snappy library will be in the build output directory
                    let snappy_lib_dir = format!("{}/build", out_dir);
                    println!("cargo:rustc-env=CMAKE_FIND_ROOT_PATH_MODE_LIBRARY=BOTH");
                    println!("cargo:rustc-env=CMAKE_FIND_ROOT_PATH_MODE_INCLUDE=BOTH");
                }
                
                // Copy libc++_shared.so to jniLibs directory for Android
                let jni_libs_dir = std::path::Path::new("gen/android/app/src/main/jniLibs");
                if jni_libs_dir.exists() {
                    use std::path::{Path, PathBuf};

                    let ndk_source = if target.contains("aarch64") {
                        Some(PathBuf::from(format!("{}/toolchains/llvm/prebuilt/darwin-x86_64/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so", ndk_home)))
                    } else if target.contains("x86_64") {
                        Some(PathBuf::from(format!("{}/toolchains/llvm/prebuilt/darwin-x86_64/sysroot/usr/lib/x86_64-linux-android/libc++_shared.so", ndk_home)))
                    } else {
                        None
                    };

                    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
                    let fallback_dir = if target.contains("aarch64") {
                        manifest_dir.join("prebuilt-libs/arm64-v8a/libc++_shared.so")
                    } else if target.contains("x86_64") {
                        manifest_dir.join("prebuilt-libs/x86_64/libc++_shared.so")
                    } else {
                        manifest_dir.join("prebuilt-libs/libc++_shared.so")
                    };

                    let mut source_candidates = Vec::new();
                    if let Some(path) = ndk_source {
                        source_candidates.push(path);
                    }
                    source_candidates.push(fallback_dir);

                    if let Some(source_path) = source_candidates.into_iter().find(|candidate| {
                        candidate.exists()
                            && candidate
                                .metadata()
                                .map(|m| m.len() > 0)
                                .unwrap_or(false)
                    }) {
                        if let Some(arch_dir) = if target.contains("aarch64") {
                            Some(jni_libs_dir.join("arm64-v8a"))
                        } else if target.contains("x86_64") {
                            Some(jni_libs_dir.join("x86_64"))
                        } else {
                            None
                        } {
                            if let Err(e) = std::fs::create_dir_all(&arch_dir) {
                                eprintln!("Warning: Failed to create jniLibs directory: {}", e);
                            } else {
                                let dest = arch_dir.join("libc++_shared.so");
                                if let Err(e) = std::fs::copy(&source_path, &dest) {
                                    eprintln!(
                                        "Warning: Failed to copy libc++_shared.so from {}: {}",
                                        source_path.display(),
                                        e
                                    );
                                } else {
                                    println!(
                                        "cargo:warning=Copied libc++_shared.so from {} to {}",
                                        source_path.display(),
                                        dest.display()
                                    );
                                }
                            }
                        }
                    } else {
                        eprintln!("Warning: Unable to locate a valid libc++_shared.so for target {}", target);
                    }
                }
            }
        }
    }
}
