fn main() {
    tauri_build::build();
    export_default_rpc_url();
    
    // Configure Android NDK environment for CMake builds
    if let Ok(target) = std::env::var("TARGET") {
        if target.contains("android") {
            // Set Android NDK environment variables for CMake
            if let Ok(ndk_home) = std::env::var("ANDROID_NDK_HOME") {
                println!("cargo:rustc-env=ANDROID_NDK_HOME={}", ndk_home);
                println!("cargo:rustc-env=ANDROID_NDK_ROOT={}", ndk_home);
                println!("cargo:rustc-link-lib=c++_static");
                
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
            }
        }
    }
}

/// Bakes `VITE_APP_API` into the binary as the default RPC endpoint, read like Vite does. The mode
/// follows the cargo profile (release = production), which `tauri build` and `tauri dev` match.
fn export_default_rpc_url() {
    const KEY: &str = "VITE_APP_API";
    let mode = match std::env::var("PROFILE").as_deref() {
        Ok("release") => "production",
        _ => "development",
    };
    println!("cargo:rerun-if-env-changed={KEY}");
    let mut from_files = None;
    for file in [".env".to_string(), ".env.local".to_string(), format!(".env.{mode}"), format!(".env.{mode}.local")] {
        let path = format!("../{file}");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => panic!("Failed to read {path}: {e}"),
        };
        println!("cargo:rerun-if-changed={path}");
        if let Some(value) = text.lines().filter_map(|line| env_value(line, KEY)).last() {
            from_files = Some(value);
        }
    }
    let url = std::env::var(KEY)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or(from_files)
        .unwrap_or_else(|| panic!("{KEY} is not set: copy .env.example to .env"));
    println!("cargo:rustc-env=XNT_DEFAULT_RPC_URL={url}");
}

/// Value of `key` on a dotenv line (`[export ]KEY=value`, optionally quoted or with a trailing
/// ` # comment`).
fn env_value(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    let line = line.strip_prefix("export ").unwrap_or(line);
    let (name, value) = line.split_once('=')?;
    if name.trim() != key {
        return None;
    }
    let value = value.trim();
    let value = match value.chars().next() {
        Some(quote @ ('"' | '\'')) => value[1..].split(quote).next().unwrap_or(""),
        _ => value.split(" #").next().unwrap_or("").trim(),
    };
    (!value.is_empty()).then(|| value.to_string())
}
