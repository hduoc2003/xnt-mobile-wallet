use std::{
    env,
    path::{Path, PathBuf},
};

#[cfg(feature = "snappy")]
const SNAPPY_VERSION: &'static str = "1.1.7";
const LEVELDB_VERSION: &'static str = "1.22";
/// Directory name within `$OUT_DIR` where the static libraries should be built.
const LIBDIR: &'static str = "lib";

#[cfg(feature = "snappy")]
fn build_snappy() -> PathBuf {
    println!("[snappy] Building");

    let outdir = env::var("OUT_DIR").unwrap();
    let libdir = Path::new(&outdir).join(LIBDIR);

    env::set_var("NUM_JOBS", num_cpus::get().to_string());
    let target = env::var("TARGET").unwrap();
    
    // Get toolchain file from environment or construct it
    let toolchain_file = if let Ok(tf) = env::var("CMAKE_TOOLCHAIN_FILE") {
        Some(tf)
    } else if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
        let tf = format!("{}/build/cmake/android.toolchain.cmake", ndk_home);
        if Path::new(&tf).exists() {
            Some(tf)
        } else {
            None
        }
    } else {
        None
    };
    
    let mut config = cmake::Config::new(Path::new("deps").join(format!("snappy-{}", SNAPPY_VERSION)));
    
    // For Android builds, set toolchain file and Android variables
    // CRITICAL: Use .env() to explicitly pass environment variables to CMake
    // AND use .define() to set them as CMake variables
    // The Android toolchain file reads from environment variables during initialization
    if target.contains("android") {
        if let Some(ref tf) = toolchain_file {
            config.define("CMAKE_TOOLCHAIN_FILE", tf);
            // Also set as environment variable so CMake can read it
            config.env("CMAKE_TOOLCHAIN_FILE", tf);
        }
        
        if target.contains("aarch64") {
            // Set as both environment variable (for toolchain file) and CMake variable
            config.env("ANDROID_ABI", "arm64-v8a");
            config.env("ANDROID_PLATFORM", "android-24");
            config.define("ANDROID_ABI", "arm64-v8a");
            config.define("ANDROID_PLATFORM", "android-24");
        } else if target.contains("x86_64") {
            config.env("ANDROID_ABI", "x86_64");
            config.env("ANDROID_PLATFORM", "android-24");
            config.define("ANDROID_ABI", "x86_64");
            config.define("ANDROID_PLATFORM", "android-24");
        }
        
        if let Ok(cc) = env::var("CC") {
            config.define("CMAKE_C_COMPILER", &cc);
        }
        if let Ok(cxx) = env::var("CXX") {
            config.define("CMAKE_CXX_COMPILER", &cxx);
        }
    }
    
    // Now set other CMake variables
    config
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("SNAPPY_BUILD_TESTS", "OFF")
        .define("HAVE_LIBZ", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", &libdir);
    
    let dest_prefix = config.build();

    assert_eq!(
        dest_prefix.join(LIBDIR),
        libdir,
        "CMake should build Snappy in provided LIBDIR"
    );
    println!("cargo:rustc-link-search=native={}", libdir.display());
    println!("cargo:rustc-link-lib=static=snappy");

    dest_prefix
}

fn build_leveldb(snappy_prefix: Option<PathBuf>) {
    println!("[leveldb] Building");

    let outdir = env::var("OUT_DIR").unwrap();
    let libdir = Path::new(&outdir).join(LIBDIR);

    env::set_var("NUM_JOBS", num_cpus::get().to_string());
    let target = env::var("TARGET").unwrap();
    
    // CRITICAL: For Android builds, set ANDROID_ABI and ANDROID_PLATFORM as GLOBAL environment variables
    // BEFORE creating the cmake::Config, because the Android toolchain file reads these during initialization
    if target.contains("android") {
        if target.contains("aarch64") {
            env::set_var("ANDROID_ABI", "arm64-v8a");
            env::set_var("ANDROID_PLATFORM", "android-24");
        } else if target.contains("x86_64") {
            env::set_var("ANDROID_ABI", "x86_64");
            env::set_var("ANDROID_PLATFORM", "android-24");
        }
    }
    
    // Get toolchain file from environment or construct it
    let toolchain_file = if let Ok(tf) = env::var("CMAKE_TOOLCHAIN_FILE") {
        Some(tf)
    } else if let Ok(ndk_home) = env::var("ANDROID_NDK_HOME") {
        let tf = format!("{}/build/cmake/android.toolchain.cmake", ndk_home);
        if Path::new(&tf).exists() {
            Some(tf)
        } else {
            None
        }
    } else {
        None
    };
    
    let mut config =
        cmake::Config::new(Path::new("deps").join(format!("leveldb-{}", LEVELDB_VERSION)));
    
    // For Android builds, set toolchain file and Android variables
    // CRITICAL: Use .env() to explicitly pass environment variables to CMake
    // AND use .define() to set them as CMake variables
    // The Android toolchain file reads from environment variables during initialization
    if target.contains("android") {
        if let Some(ref tf) = toolchain_file {
            config.define("CMAKE_TOOLCHAIN_FILE", tf);
            // Also set as environment variable so CMake can read it
            config.env("CMAKE_TOOLCHAIN_FILE", tf);
        }
        
        if target.contains("aarch64") {
            // Set as both environment variable (for toolchain file) and CMake variable
            config.env("ANDROID_ABI", "arm64-v8a");
            config.env("ANDROID_PLATFORM", "android-24");
            config.define("ANDROID_ABI", "arm64-v8a");
            config.define("ANDROID_PLATFORM", "android-24");
        } else if target.contains("x86_64") {
            config.env("ANDROID_ABI", "x86_64");
            config.env("ANDROID_PLATFORM", "android-24");
            config.define("ANDROID_ABI", "x86_64");
            config.define("ANDROID_PLATFORM", "android-24");
        }
        
        if let Ok(cc) = env::var("CC") {
            config.define("CMAKE_C_COMPILER", &cc);
        }
        if let Ok(cxx) = env::var("CXX") {
            config.define("CMAKE_CXX_COMPILER", &cxx);
        }
    }
    
    // Now set other CMake variables
    config
        .define("LEVELDB_BUILD_TESTS", "OFF")
        .define("LEVELDB_BUILD_BENCHMARKS", "OFF")
        .define("CMAKE_INSTALL_LIBDIR", &libdir)
        .always_configure(true); // Force CMake to reconfigure to pick up Android variables
    if let Some(ref snappy_prefix) = snappy_prefix {
        let snappy_lib_dir = snappy_prefix.join(LIBDIR);
        let snappy_lib = snappy_lib_dir.join("libsnappy.a");
        
        // Set CMAKE_LIBRARY_PATH so CMake's check_library_exists can find snappy
        config.define("CMAKE_LIBRARY_PATH", snappy_lib_dir.display().to_string());
        
        #[cfg(target_env = "msvc")]
        let ldflags = format!("/LIBPATH:{}", snappy_lib_dir.display());
        #[cfg(not(target_env = "msvc"))]
        let ldflags = format!("-L{}", snappy_lib_dir.display());
    
        env::set_var("LDFLAGS", ldflags);

        config
            .define("HAVE_SNAPPY", "ON")
            .cflag(format!("-I{}", snappy_prefix.join("include").display()))
            .cxxflag(format!("-I{}", snappy_prefix.join("include").display()));
    } else {
        config.define("HAVE_SNAPPY", "OFF");
    }
    let dest_prefix = config.build();

    assert_eq!(
        dest_prefix.join(LIBDIR),
        libdir,
        "CMake should build LevelDB in provided LIBDIR"
    );
    println!("cargo:rustc-link-search=native={}", libdir.display());
    println!("cargo:rustc-link-lib=static=leveldb");
}

fn main() {
    println!("[build] Started");

    // If we have the appropriate feature, then we build snappy.
    #[cfg(feature = "snappy")]
    let snappy_prefix = Some(build_snappy());
    #[cfg(not(feature = "snappy"))]
    let snappy_prefix: Option<PathBuf> = None;

    // Build LevelDB
    build_leveldb(snappy_prefix);

    // Link to the standard C++ library
    let target = env::var("TARGET").unwrap();
    if target.contains("apple") || target.contains("freebsd") {
        println!("cargo:rustc-link-lib=c++");
    } else if target.contains("gnu") || target.contains("netbsd") || target.contains("openbsd") {
        println!("cargo:rustc-link-lib=stdc++");
    } else if target.contains("musl") {
        // We want to link to libstdc++ *statically*. This requires that the user passes the right
        // search path to rustc via `-Lstatic=/path/to/libstdc++`.
        println!("cargo:rustc-link-lib=static=stdc++");
    }

    println!("[build] Finished");
}
