// The prebuilt native library: the shim and Elegance in one shared library (native/CMakeLists.txt),
// so apps need no C++ compiler, CMake, or vcpkg.
//
// What goes in it is pinned by this crate's files: the shim sources, which define the ABI the Rust
// side declares, and native/elegance-rev, the Elegance commit it's built from. Their hash names the
// prebuilt library: CI (.github/workflows/native.yml) attaches one archive per target to a
// prerelease tagged native-<first 12 hex digits of the hash>, and a vX.Y.Z release for each
// version. So a checkout always gets the library built from its own shim and pinned Elegance, and
// a new pin (update-elegance.yml moves it after publishing that library) means a new download.
//
// ELEGANCE_NATIVE_DIR     an extracted prebuilt archive to use (offline, or a build of your own)
// ELEGANCE_PREBUILT=0     never download one: build Elegance from source
// ELEGANCE_PREBUILT=1     always use one, even beside an Elegance checkout; fail if there's none
// ELEGANCE_PREBUILT_URL   the folder of archives (default: the GitHub prerelease for this hash)
//
// Without ELEGANCE_PREBUILT, the prebuilt library is used unless a source build was asked for
// (ELEGANCE_ROOT, ELEGANCE_BUILD_DIR, or an Elegance checkout beside this workspace), and when
// there's none for this hash and target yet, Elegance is built from source.

use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Targets the release workflow (.github/workflows/native.yml) builds.
const TARGETS: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
];

/// The pinned Elegance commit (update-elegance.yml moves it).
const ELEGANCE_REV: &str = "native/elegance-rev";

/// The files whose contents make up the prebuilt library's hash, in this order
/// (native/CMakeLists.txt hashes the same files the same way).
const HASHED: &[&str] = &[
    "shim/shim.h",
    "shim/core.cpp",
    "shim/views.cpp",
    "shim/ext.cpp",
    "shim/platform.cpp",
    ELEGANCE_REV,
];

/// The Elegance commit native/elegance-rev pins, which source builds clone too.
pub fn pinned_elegance(manifest: &Path) -> String {
    println!("cargo:rerun-if-changed={ELEGANCE_REV}");
    let path = manifest.join(ELEGANCE_REV);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("elegance-sys: can't read {}: {e}", path.display()));
    text.trim().to_owned()
}

/// The folder of a prebuilt library to link, or None to build from source.
pub fn find(manifest: &Path) -> Option<PathBuf> {
    for var in ["ELEGANCE_NATIVE_DIR", "ELEGANCE_PREBUILT", "ELEGANCE_PREBUILT_URL"] {
        println!("cargo:rerun-if-env-changed={var}");
    }
    for file in HASHED {
        println!("cargo:rerun-if-changed={file}");
    }
    if let Some(dir) = env::var_os("ELEGANCE_NATIVE_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let mode = env::var("ELEGANCE_PREBUILT").unwrap_or_default();
    if mode == "0" {
        return None;
    }
    let required = mode == "1";
    if !required && source_requested(manifest) {
        return None;
    }
    let target = env::var("TARGET").unwrap();
    let result = if TARGETS.contains(&target.as_str()) {
        download(&target, &native_hash(manifest))
    } else {
        Err(format!("there's no prebuilt Elegance for {target}"))
    };
    match result {
        Ok(dir) => Some(dir),
        Err(error) if required => panic!("\n\nelegance-sys: {error} (ELEGANCE_PREBUILT=1).\n"),
        Err(error) => {
            println!("cargo:warning={error}; building Elegance from source instead");
            None
        }
    }
}

fn source_requested(manifest: &Path) -> bool {
    ["ELEGANCE_ROOT", "ELEGANCE_BUILD_DIR"]
        .iter()
        .any(|var| env::var_os(var).is_some_and(|v| !v.is_empty()))
        || manifest
            .join("..")
            .join("..")
            .join("elegance")
            .join("CMakeLists.txt")
            .is_file()
}

/// SHA-256 of the hashed files, without carriage returns (a Windows checkout may have them).
pub fn native_hash(manifest: &Path) -> String {
    let mut hasher = Sha256::new();
    for file in HASHED {
        let path = manifest.join(file);
        let bytes =
            fs::read(&path).unwrap_or_else(|e| panic!("elegance-sys: can't read {}: {e}", path.display()));
        hasher.update(bytes.into_iter().filter(|&b| b != b'\r').collect::<Vec<_>>());
    }
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// The archive for `target`, downloaded and extracted once into the user cache folder (one folder
/// per hash): from ELEGANCE_PREBUILT_URL, else this version's release, else the prerelease for this
/// hash.
fn download(target: &str, hash: &str) -> Result<PathBuf, String> {
    let short = &hash[..12];
    let cache = crate::cmake::cache_dir()
        .ok_or("no user cache folder to put the prebuilt Elegance in")?
        .join("elegance")
        .join("native");
    let dir = |name: &str| cache.join(format!("{name}-{short}")).join(target);
    let file = format!("elegance-native-{target}.tar.gz");
    if let Some(base) = env::var("ELEGANCE_PREBUILT_URL").ok().filter(|u| !u.is_empty()) {
        return fetch(&dir("custom"), &format!("{}/{file}", base.trim_end_matches('/')), hash);
    }
    let releases = "https://github.com/EleganceUI/elegance-rs/releases/download";
    let version = format!("v{}", env::var("CARGO_PKG_VERSION").unwrap());
    let tag = format!("native-{short}");
    // The release's archive only counts if its hash matches (a git checkout past the release).
    fetch(&dir(&version), &format!("{releases}/{version}/{file}"), hash)
        .or_else(|_| fetch(&dir("native"), &format!("{releases}/{tag}/{file}"), hash))
}

/// `url`'s archive, extracted into `dir` (unless it's there already) if its hash matches.
fn fetch(dir: &Path, url: &str, hash: &str) -> Result<PathBuf, String> {
    let dir = dir.to_owned();
    if matches_hash(&dir, hash) {
        return Ok(dir);
    }
    let partial = dir.with_extension("partial");
    let _ = fs::remove_dir_all(&partial);
    fs::create_dir_all(&partial).map_err(|e| format!("can't create {}: {e}", partial.display()))?;
    let archive = partial.join("archive.tar.gz");
    run(
        Command::new(system_tool("curl")).args(["-fsSL", "--retry", "3", "-o"]).arg(&archive).arg(url),
        &format!("downloading {url} (CI may not have built this revision yet)"),
    )?;
    run(
        Command::new(system_tool("tar")).arg("-xzf").arg(&archive).arg("-C").arg(&partial),
        &format!("extracting {}", archive.display()),
    )?;
    let _ = fs::remove_file(&archive);
    if !matches_hash(&partial, hash) {
        let _ = fs::remove_dir_all(&partial);
        return Err(format!(
            "the prebuilt Elegance at {url} was built from a different shim or Elegance than this elegance-sys pins"
        ));
    }
    let _ = fs::remove_dir_all(&dir);
    fs::rename(&partial, &dir).map_err(|e| format!("can't move the prebuilt Elegance to {}: {e}", dir.display()))?;
    println!("cargo:warning=using the prebuilt Elegance from {url}");
    Ok(dir)
}

// curl and tar come with Windows 10 and later, macOS, and practically every Linux. On Windows, use
// the system's own: Git for Windows puts GNU tar on PATH, which takes `C:\...` for a remote host.
fn system_tool(name: &str) -> PathBuf {
    if cfg!(windows)
        && let Some(root) = env::var_os("SystemRoot") {
            let exe = Path::new(&root).join("System32").join(format!("{name}.exe"));
            if exe.is_file() {
                return exe;
            }
        }
    PathBuf::from(name)
}

fn matches_hash(dir: &Path, hash: &str) -> bool {
    fs::read_to_string(dir.join("native.sha256")).is_ok_and(|text| text.trim() == hash) && dir.join("lib").is_dir()
}

fn run(command: &mut Command, what: &str) -> Result<(), String> {
    let status = command
        .stdout(Stdio::null())
        .status()
        .map_err(|e| format!("{what} failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{what} failed ({status})"))
    }
}

/// The library in an extracted archive's lib/: the import library on Windows.
pub fn library(native: &Path) -> PathBuf {
    let lib = native.join("lib");
    ["elegance_native.lib", "libelegance_native.so", "libelegance_native.dylib"]
        .iter()
        .map(|file| lib.join(file))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("\n\nelegance-sys: no elegance_native library in {}.\n", lib.display()))
}
