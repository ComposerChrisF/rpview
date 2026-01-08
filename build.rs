use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!("cargo:rerun-if-changed=build.rs");

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-env=TARGET_PLATFORM=macOS");
            // Future: Add macOS-specific resources, icons, plist files
            // println!("cargo:rustc-link-arg=-mmacosx-version-min=10.15");
        }
        "windows" => {
            println!("cargo:rustc-env=TARGET_PLATFORM=Windows");
            // Future: Add Windows-specific resources, icons, manifest
            // Set Windows subsystem to avoid console window
            println!("cargo:rustc-link-arg=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg=/ENTRY:mainCRTStartup");
        }
        "linux" => {
            println!("cargo:rustc-env=TARGET_PLATFORM=Linux");
            // Future: Add Linux-specific resources, .desktop files
        }
        _ => {
            println!("cargo:rustc-env=TARGET_PLATFORM=Unknown");
        }
    }
}
