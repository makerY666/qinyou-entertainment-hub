fn main() {
    // GNU windres cannot read icon paths containing non-ASCII characters.
    // Use the build output path so Chinese-named source workspaces remain usable.
    let mut attributes = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let icon =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("app-icon.ico");
        std::fs::copy("icons/icon.ico", &icon).expect("copy application icon");
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new().window_icon_path(icon));
        println!("cargo:rerun-if-changed=icons/icon.ico");
    }
    tauri_build::try_build(attributes).expect("generate native application resources");
}
