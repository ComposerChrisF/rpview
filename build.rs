use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();

    println!("cargo:rerun-if-changed=build.rs");

    match target_os.as_str() {
        "macos" => {
            println!("cargo:rustc-env=TARGET_PLATFORM=macOS");
        }
        "windows" => {
            println!("cargo:rustc-env=TARGET_PLATFORM=Windows");
            // Set Windows subsystem to avoid console window (bins only, not tests)
            println!("cargo:rustc-link-arg-bins=/SUBSYSTEM:WINDOWS");
            println!("cargo:rustc-link-arg-bins=/ENTRY:mainCRTStartup");

            // Embed icon and version resource into the executable
            println!("cargo:rerun-if-changed=packaging/windows/rpview.ico");
            embed_windows_resource();
        }
        "linux" => {
            println!("cargo:rustc-env=TARGET_PLATFORM=Linux");
        }
        _ => {
            println!("cargo:rustc-env=TARGET_PLATFORM=Unknown");
        }
    }
}

#[cfg(target_os = "windows")]
fn embed_windows_resource() {
    let ico_path = "packaging/windows/rpview.ico";
    if !std::path::Path::new(ico_path).exists() {
        println!("cargo:warning=Icon file not found at {ico_path}, skipping resource embedding");
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path);
    res.set("ProductName", "rpview");
    res.set("FileDescription", "A fast, cross-platform image viewer");
    res.set("LegalCopyright", "Copyright (c) rpview contributors");
    res.compile().expect("Failed to compile Windows resource");
}

#[cfg(not(target_os = "windows"))]
fn embed_windows_resource() {
    // No-op on non-Windows platforms
}
