fn main() {
    println!("cargo:rerun-if-env-changed=AETHER_TEST_BUILD");

    let mut windows = tauri_build::WindowsAttributes::new();

    if std::env::var("AETHER_TEST_BUILD").is_err() {
        windows = windows.app_manifest(include_str!("windows-app-manifest.xml"));
    }

    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run tauri_build");
}
