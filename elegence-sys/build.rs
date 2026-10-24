// Builds the C++ shim against an Elegence build tree and links elegence.dll.
//
// ELEGENCE_ROOT       the Elegence source tree (default: ../../ElegenceMain, beside this workspace)
// ELEGENCE_BUILD_DIR  a configured, built tree (default: $ELEGENCE_ROOT/build/dev-release)
//
// The shim passes std::string and std::function across the DLL boundary, so it must use the same
// compiler and C runtime as the DLL. Rust links the release CRT (/MD), so use a release-CRT build
// of Elegence (the dev-release preset), not dev-debug (/MDd).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const SHIM_FILES: &[&str] = &["shim/core.cpp", "shim/views.cpp", "shim/ext.cpp", "shim/platform.cpp"];

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = env::var_os("ELEGENCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("..").join("..").join("ElegenceMain"));
    let build = env::var_os("ELEGENCE_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("build").join("dev-release"));

    println!("cargo:rerun-if-env-changed=ELEGENCE_ROOT");
    println!("cargo:rerun-if-env-changed=ELEGENCE_BUILD_DIR");
    println!("cargo:rerun-if-changed=shim/shim.h");
    for file in SHIM_FILES {
        println!("cargo:rerun-if-changed={file}");
    }

    let import_lib = build.join("lib").join("elegence.lib");
    if !import_lib.exists() {
        panic!(
            "\n\nelegence-sys: {} not found.\nBuild Elegence first (from a VS developer prompt, in {}):\n    \
             cmake --preset dev-release\n    cmake --build build/dev-release --target elegence\nor set \
             ELEGENCE_BUILD_DIR to a built tree that uses the release C runtime (/MD).\n",
            import_lib.display(),
            root.display()
        );
    }
    let generated = build.join("src").join("generated");
    if !generated.join("elg").join("export.h").exists() {
        panic!("elegence-sys: {} has no generated export header", build.display());
    }

    let mut shim = cc::Build::new();
    shim.cpp(true)
        .files(SHIM_FILES)
        .include(root.join("include"))
        .include(&generated)
        .define("NOMINMAX", None)
        .static_crt(false)
        .warnings(false);
    if shim.get_compiler().is_like_msvc() {
        shim.flag("/std:c++latest").flag("/EHsc").flag("/permissive-").flag("/utf-8").flag("/Zc:__cplusplus");
    } else {
        shim.flag("-std=c++23");
    }
    shim.compile("elegence_shim");

    println!("cargo:rustc-link-search=native={}", build.join("lib").display());
    println!("cargo:rustc-link-lib=dylib=elegence");
    println!("cargo:root={}", root.display());
    println!("cargo:build_dir={}", build.display());

    copy_runtime_dlls(&build.join("bin"));
}

// Windows finds DLLs next to the executable: copy elegence.dll and its dependencies to where
// cargo puts binaries, tests, and examples.
fn copy_runtime_dlls(bin: &Path) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // OUT_DIR is target/<profile>/build/<crate>-<hash>/out.
    let Some(profile_dir) = out_dir.ancestors().nth(3) else { return };
    let Ok(entries) = fs::read_dir(bin) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("dll")) {
            println!("cargo:rerun-if-changed={}", path.display());
            for dir in [profile_dir.to_path_buf(), profile_dir.join("deps"), profile_dir.join("examples")] {
                let _ = fs::create_dir_all(&dir);
                let _ = fs::copy(&path, dir.join(path.file_name().unwrap()));
            }
        }
    }
}
