fn main() {
    println!("cargo:rerun-if-env-changed=AETHER_TEST_BUILD");

    bundle_gnullvm_runtime();

    let mut windows = tauri_build::WindowsAttributes::new();

    if std::env::var("AETHER_TEST_BUILD").is_err() {
        windows = windows.app_manifest(include_str!("windows-app-manifest.xml"));
    }

    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run tauri_build");
}

fn bundle_gnullvm_runtime() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.ends_with("windows-gnullvm") {
        return;
    }

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(rustc)
        .args(["--print", "sysroot"])
        .output()
        .expect("failed to locate the Rust sysroot");
    assert!(output.status.success(), "rustc --print sysroot failed");

    let sysroot = String::from_utf8(output.stdout)
        .expect("Rust sysroot path was not UTF-8")
        .trim()
        .to_owned();
    let source = std::path::Path::new(&sysroot)
        .join("lib")
        .join("rustlib")
        .join(&target)
        .join("bin")
        .join("libunwind.dll");
    let destination = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("target")
        .join("runtime")
        .join("libunwind.dll");

    let profile_destination = std::path::Path::new(&std::env::var("OUT_DIR").unwrap())
        .ancestors()
        .nth(3)
        .expect("Cargo OUT_DIR did not contain a profile directory")
        .join("libunwind.dll");

    let dependency_destination = profile_destination
        .parent()
        .unwrap()
        .join("deps")
        .join("libunwind.dll");

    for destination in [destination, profile_destination, dependency_destination] {
        stage_runtime_file(&source, &destination);
    }
}

fn stage_runtime_file(source: &std::path::Path, destination: &std::path::Path) {
    std::fs::create_dir_all(destination.parent().unwrap())
        .expect("failed to create a runtime directory");

    let source_contents = std::fs::read(source)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", source.display()));
    if let Ok(existing_contents) = std::fs::read(destination) {
        if existing_contents == source_contents {
            return;
        }
    }

    std::fs::write(destination, source_contents).unwrap_or_else(|error| {
        panic!(
            "failed to stage {} at {}: {error}",
            source.display(),
            destination.display()
        )
    });
}
