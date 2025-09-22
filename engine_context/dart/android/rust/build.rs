fn main() {
    // Only add linker args when targeting Android.
    // This adds support for 16KB page sizes by instructing the linker.
    // Some Android NDKs/ld versions accept -z,max-page-size; older ones may need common-page-size.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        // Prefer max-page-size which is supported by modern linkers.
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
        // Add a fallback linker flag that may be required for some older toolchains.
        println!("cargo:rustc-link-arg=-Wl,-z,common-page-size=16384");
    }
}
