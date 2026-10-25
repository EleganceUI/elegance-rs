// Builds the C++ shim against an Elegance build tree and links Elegance.
//
// By default Elegance is linked statically, from a static build (the dev-release-static preset):
// the app is one executable plus the DLLs Elegance's dependencies come in. The `dynamic` feature
// links elegance.dll from a shared build (dev-release) instead, which the preview host and hot
// reload need: a views module and the host it loads into must share one Elegance.
//
// ELEGANCE_ROOT               the Elegance source tree (default: ../../elegance, a checkout beside this
//                             workspace; without one, a clone of github.com/EleganceUI/elegance, see below)
// ELEGANCE_GIT_REF            the branch or tag that clone checks out (default: main)
// ELEGANCE_CACHE_DIR          where clones go (default: the user cache folder's elegance/)
// ELEGANCE_BUILD_DIR          a static build (default: $ELEGANCE_ROOT/build/dev-release-static if built)
// ELEGANCE_SHARED_BUILD_DIR   a shared build, for `dynamic` (default: $ELEGANCE_ROOT/build/dev-release if built)
// ELEGANCE_AUTO_BUILD=0       fail instead of building Elegance
//
// Without a build tree, this script builds Elegance itself with CMake and vcpkg, into
// $ELEGANCE_ROOT/build/cargo-release-static (or cargo-release); see build_cmake.rs for how it
// finds (or bootstraps) vcpkg, CMake, Ninja, and the compiler. Without a source tree, it clones
// Elegance once into the user cache folder (%LOCALAPPDATA%\elegance\src or ~/.cache/elegance/src),
// where every project that uses these bindings shares it and its build.
//
// The shim passes std::string and std::function across to Elegance, so it must use the same
// compiler and C runtime. Rust links the release CRT (/MD), so use release-CRT builds of
// Elegance, not dev-debug (/MDd).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "build_cmake.rs"]
mod cmake;
mod build_runtime;

const SHIM_FILES: &[&str] = &["shim/core.cpp", "shim/views.cpp", "shim/ext.cpp", "shim/platform.cpp"];

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = elegance_root(&manifest);
    let dynamic = env::var_os("CARGO_FEATURE_DYNAMIC").is_some();
    let (build_var, preset) = if dynamic {
        ("ELEGANCE_SHARED_BUILD_DIR", "dev-release")
    } else {
        ("ELEGANCE_BUILD_DIR", "dev-release-static")
    };
    println!("cargo:rerun-if-env-changed={build_var}");
    println!("cargo:rerun-if-env-changed=ELEGANCE_AUTO_BUILD");
    println!("cargo:rerun-if-changed=shim/shim.h");
    for file in SHIM_FILES {
        println!("cargo:rerun-if-changed={file}");
    }

    // An explicit build tree, else the preset's if it's built, else one this script builds.
    let build = match env::var_os(build_var).filter(|d| !d.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None if is_built(&root.join("build").join(preset), dynamic) => root.join("build").join(preset),
        None if env::var("ELEGANCE_AUTO_BUILD").is_ok_and(|v| v == "0") => {
            missing_build(&root.join("build").join(preset), &root, preset, build_var)
        }
        None => {
            let build = root.join("build").join(if dynamic { "cargo-release" } else { "cargo-release-static" });
            cmake::build_elegance(&root, &build, dynamic);
            build
        }
    };

    let generated = build.join("src").join("generated");
    if !generated.join("elg").join("export.h").exists() {
        missing_build(&build, &root, preset, build_var);
    }
    let link = if dynamic { Link::shared(&build) } else { Link::from_manifest(&build) };
    let link = link.unwrap_or_else(|| missing_build(&build, &root, preset, build_var));

    let mut shim = cc::Build::new();
    shim.cpp(true)
        .files(SHIM_FILES)
        .include(root.join("include"))
        .include(&generated)
        .define("NOMINMAX", None)
        .static_crt(false)
        .warnings(false);
    for define in &link.defines {
        match define.split_once('=') {
            Some((name, value)) => shim.define(name, value),
            None => shim.define(define, None),
        };
    }
    if shim.get_compiler().is_like_msvc() {
        shim.flag("/std:c++latest").flag("/EHsc").flag("/permissive-").flag("/utf-8").flag("/Zc:__cplusplus");
    } else {
        shim.flag("-std=c++23");
    }
    shim.compile("elegance_shim");

    for (kind, lib) in &link.libs {
        link_lib(kind, lib);
    }
    println!("cargo:root={}", root.display());
    println!("cargo:build_dir={}", build.display());
    println!("cargo:dynamic={dynamic}");

    copy_runtime_files(&link.runtime);
}

// ELEGANCE_ROOT, else a checkout beside this workspace, else a clone in the user cache folder.
fn elegance_root(manifest: &Path) -> PathBuf {
    println!("cargo:rerun-if-env-changed=ELEGANCE_ROOT");
    println!("cargo:rerun-if-env-changed=ELEGANCE_GIT_REF");
    if let Some(root) = env::var_os("ELEGANCE_ROOT").filter(|r| !r.is_empty()) {
        return PathBuf::from(root);
    }
    let sibling = manifest.join("..").join("..").join("elegance");
    if sibling.join("CMakeLists.txt").is_file() {
        return sibling;
    }
    let reference = env::var("ELEGANCE_GIT_REF").ok().filter(|r| !r.is_empty()).unwrap_or_else(|| "main".to_owned());
    cmake::clone_elegance(&reference)
}

fn is_built(build: &Path, dynamic: bool) -> bool {
    let library = if dynamic {
        build_runtime::shared_library(build).is_some()
            && build.join("bin").join(format!("elegance-preview{}", env::consts::EXE_SUFFIX)).exists()
    } else {
        build.join("elegance-link.txt").exists()
    };
    library && build.join("src/generated/elg/export.h").exists()
}

fn missing_build(build: &Path, root: &Path, preset: &str, build_var: &str) -> ! {
    let target = if preset == "dev-release" { "elegance elegance-preview" } else { "elegance" };
    panic!(
        "\n\nelegance-sys: no usable Elegance build at {}.\nBuild Elegance first (from a VS developer prompt, in {}):\n    \
         cmake --preset {preset}\n    cmake --build build/{preset} --target {target}\nor set {build_var} to a \
         built tree that uses the release C runtime (/MD).\nWithout {build_var} or ELEGANCE_AUTO_BUILD=0, \
         elegance-sys builds Elegance itself.\n",
        build.display(),
        root.display()
    );
}

#[derive(Default)]
struct Link {
    /// (kind, name or file): "static" or "lib".
    libs: Vec<(&'static str, String)>,
    defines: Vec<String>,
    /// Shared libraries and preview hosts, or folders containing them, to copy beside the binaries.
    runtime: Vec<PathBuf>,
}

impl Link {
    // A shared build: the import library on Windows, the shared library elsewhere.
    // Its bin folder also holds the preview host.
    fn shared(build: &Path) -> Option<Link> {
        let file = build_runtime::shared_library(build)?;
        let mut runtime = vec![build.join("bin")];
        if file.extension().is_some_and(|ext| ext != "lib") && file.parent() != Some(build.join("bin").as_path()) {
            runtime.push(file.clone());
        }
        Some(Link {
            libs: vec![("lib", file.display().to_string())],
            defines: Vec::new(),
            runtime,
        })
    }

    // A static build: elegance-link.txt, written by Elegance's CMake (cmake/ElgLinkManifest.cmake).
    fn from_manifest(build: &Path) -> Option<Link> {
        let file = build.join("elegance-link.txt");
        println!("cargo:rerun-if-changed={}", file.display());
        let text = fs::read_to_string(&file).ok()?;
        let mut link = Link::default();
        for line in text.lines() {
            let Some((kind, value)) = line.trim().split_once(' ') else {
                continue;
            };
            let value = value.trim();
            if value.is_empty() {
                continue;
            }
            match kind {
                "static" if is_rust_staticlib(value) => panic!(
                    "\n\nelegance-sys: {} links AccessKit's static library, a Rust staticlib whose own Rust \
                     standard library clashes with the app's.\nReconfigure it with -DELG_ACCESSKIT_SHARED=ON (the \
                     dev-release-static preset sets it).\n",
                    build.display()
                ),
                "static" => link.libs.push(("static", value.to_owned())),
                "lib" => link.libs.push(("lib", value.to_owned())),
                "runtime" | "runtime-dir" => link.runtime.push(PathBuf::from(value)),
                "define" => link.defines.push(value.to_owned()),
                _ => {}
            }
        }
        Some(link)
    }
}

fn is_rust_staticlib(file: &str) -> bool {
    Path::new(file).file_stem().is_some_and(|stem| {
        let stem = stem.to_string_lossy();
        stem == "accesskit" || stem == "libaccesskit"
    })
}

// A library file becomes a search folder and a name; a bare name is a system library.
fn link_lib(kind: &str, lib: &str) {
    let path = Path::new(lib);
    let framework = lib.strip_prefix("-framework ")
        .or_else(|| lib.strip_prefix("-Wl,-framework,"))
        .or_else(|| lib.strip_prefix("LINKER:-framework,"))
        .map(str::trim)
        .or_else(|| (path.extension().is_some_and(|ext| ext == "framework"))
            .then(|| path.file_stem().and_then(|name| name.to_str())).flatten());
    if let Some(framework) = framework {
        println!("cargo:rustc-link-lib=framework={framework}");
        if path.is_absolute() {
            println!("cargo:rustc-link-search=framework={}", path.parent().unwrap().display());
        }
        return;
    }
    let name = if path.is_absolute() {
        println!("cargo:rerun-if-changed={lib}");
        println!("cargo:rustc-link-search=native={}", path.parent().unwrap().display());
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        // libfoo.a / libfoo.so link as `foo`; on Windows the file name is the name.
        match (cfg!(windows), stem.strip_prefix("lib")) {
            (false, Some(rest)) => rest.to_owned(),
            _ => stem,
        }
    } else {
        lib.trim_start_matches("-l").trim_end_matches(".lib").to_owned()
    };
    if kind == "static" {
        // Linked into the final binary, not copied into this crate's rlib (it's large).
        println!("cargo:rustc-link-lib=static:-bundle={name}");
    } else {
        println!("cargo:rustc-link-lib=dylib={name}");
    }
}

// Copy runtime libraries and the preview host to where Cargo puts binaries, tests, and examples.
fn copy_runtime_files(runtime: &[PathBuf]) {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    // OUT_DIR is target/<profile>/build/<crate>-<hash>/out.
    let Some(profile_dir) = out_dir.ancestors().nth(3) else {
        return;
    };
    let mut files = Vec::new();
    for item in runtime {
        if item.is_dir() {
            let Ok(entries) = fs::read_dir(item) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().unwrap().to_string_lossy();
                if path.is_file() && build_runtime::is_runtime_file(&name) {
                    files.push(path);
                }
            }
        } else if item.is_file() {
            files.push(item.clone());
        }
    }
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        for dir in [profile_dir.to_path_buf(), profile_dir.join("deps"), profile_dir.join("examples")] {
            let _ = fs::create_dir_all(&dir);
            let _ = fs::copy(&path, dir.join(path.file_name().unwrap()));
        }
    }
}
