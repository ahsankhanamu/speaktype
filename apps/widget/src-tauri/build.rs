fn main() {
    tauri_build::build();
    // The apple-vision crate links Swift, and the Swift UI/Vision runtime is
    // referenced as @rpath/libswift_Concurrency.dylib at link time. Without an
    // LC_RPATH the binary dies at launch with "Library not loaded:
    // @rpath/libswift_Concurrency.dylib … no LC_RPATH's found". /usr/lib/swift
    // is the Apple-sanctioned location macOS resolves from its dyld shared
    // cache (same path every other libswift* reference resolves from).
    //
    // This is emitted for BIN and TEST targets (the test harness is its own
    // target and would otherwise still need DYLD_LIBRARY_PATH, which loads the
    // Xcode toolchain's second copy of libswift_Concurrency and triggers the
    // noisy "Class SwiftNativeNSObject is implemented in both …" warning).
    #[cfg(target_os = "macos")]
    {
        // rustc-link-arg (no suffix) covers every target, including the lib
        // unit-test harness, so DYLD_LIBRARY_PATH is never needed.
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    }
}
