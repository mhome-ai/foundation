fn main() {
    println!("cargo:rerun-if-changed=native/macos_permissions.m");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    cc::Build::new()
        .file("native/macos_permissions.m")
        .flag("-fobjc-arc")
        .flag("-fmodules")
        .compile("os_permissions_macos");
    println!("cargo:rustc-link-lib=framework=CoreBluetooth");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=EventKit");
    println!("cargo:rustc-link-lib=framework=CoreServices");
}
