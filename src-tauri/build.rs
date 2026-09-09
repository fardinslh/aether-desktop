fn main() {
    println!("cargo:rerun-if-env-changed=AETHER_TEST_BUILD");

    bundle_gnullvm_runtime();

    let test_build = std::env::var("AETHER_TEST_BUILD").is_ok();
    if test_build && std::env::var("PROFILE").as_deref() == Ok("release") {
        println!(
            "cargo:warning=AETHER_TEST_BUILD is set in a release-profile build: binaries receive the asInvoker test manifest and silently lose requireAdministrator elevation. Unset AETHER_TEST_BUILD for production, installer, and NSIS builds."
        );
    }

    let windows = if test_build {
        // Test builds must never request elevation, and exactly one manifest
        // source must exist: embed_windows_test_manifest links the
        // Common-Controls v6 asInvoker manifest into every artifact, so
        // tauri-build's own manifest is disabled here to avoid a duplicate
        // RT_MANIFEST resource racing for resource id 1.
        tauri_build::WindowsAttributes::new_without_app_manifest()
    } else {
        tauri_build::WindowsAttributes::new()
            .app_manifest(include_str!("windows-app-manifest.xml"))
    };

    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run tauri_build");

    embed_windows_test_manifest(test_build);
}

fn embed_windows_test_manifest(link_every_artifact: bool) {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }

    // tauri-build embeds the app manifest only into bin targets through
    // `cargo:rustc-link-arg-bins`. Test harness binaries (lib unittests and
    // integration tests) link the same code, which imports Common-Controls
    // v6-only entry points such as `TaskDialogIndirect`. Without a manifest
    // the Windows loader resolves comctl32.dll to version 5 and aborts the
    // process with STATUS_ENTRYPOINT_NOT_FOUND before the harness starts.
    // Test binaries must never request elevation, so this manifest is
    // asInvoker.
    //
    // AETHER_TEST_BUILD=1 links the manifest into every artifact (bins, lib
    // unittests, integration tests, doctests) because tauri's own manifest is
    // disabled in that mode. Production builds keep the requireAdministrator
    // app manifest on bins and link this manifest into integration tests
    // only; the documented isolated-test flow always sets AETHER_TEST_BUILD.
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("no OUT_DIR"));
    let rc = out_dir.join("aether-test-manifest.rc");
    let manifest = include_str!("windows-test-manifest.xml");

    let mut rc_content = String::from("1 24\n{\n");
    for line in manifest.lines() {
        let escaped = line.trim().replace('\\', "\\\\").replace('"', "\"\"");
        rc_content.push_str(&format!("\" {escaped} \"\n"));
    }
    rc_content.push_str("}\n");
    std::fs::write(&rc, rc_content)
        .unwrap_or_else(|error| panic!("failed to write {}: {error}", rc.display()));

    let result = if link_every_artifact {
        embed_resource::compile_for_everything(&rc, embed_resource::NONE)
    } else {
        embed_resource::compile_for_tests(&rc, embed_resource::NONE)
    };

    if result.manifest_required().is_err() {
        println!("cargo:warning=Windows test manifest resource was not compiled; test binaries may fail to start with STATUS_ENTRYPOINT_NOT_FOUND");
    }
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
