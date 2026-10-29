// The prebuilt C library: Elegance's C API (its c/ folder) and Elegance in one shared library, so
// apps need no C++ compiler, CMake, or vcpkg.
//
// elegance-rev pins the Elegance commit whose C API this crate declares. Elegance's CI
// (.github/workflows/c-api.yml there) builds the library for every commit on its main and attaches
// one archive per target to a prerelease tagged c-<first 12 hex digits of the commit>. So a
// checkout always gets the library its pin names, and moving the pin (bump-elegance.yml opens a
// pull request for that) means a new download.
//
// ELEGANCE_NATIVE_DIR     an extracted archive to use (offline, or a build of your own)
// ELEGANCE_PREBUILT=0     never download one: build Elegance from source
// ELEGANCE_PREBUILT=1     always use one, even beside an Elegance checkout; fail if there's none
// ELEGANCE_PREBUILT_URL   the folder of archives (default: the prerelease for the pinned commit)
//
// Without ELEGANCE_PREBUILT, the prebuilt library is used unless a source build was asked for
// (ELEGANCE_ROOT, ELEGANCE_BUILD_DIR, or an Elegance checkout beside this workspace), and when
// there's none for this commit and target, Elegance is built from source.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Targets Elegance's CI builds the library for.
const TARGETS: &[&str] = &[
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
];

/// The pinned Elegance commit.
const ELEGANCE_REV: &str = "elegance-rev";

/// The Elegance commit elegance-rev pins, which source builds clone too.
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
        download(&target, &pinned_elegance(manifest))
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

/// The archive for `target` and Elegance commit `rev`, downloaded and extracted once into the user
/// cache folder (one folder per commit): from ELEGANCE_PREBUILT_URL, else that commit's prerelease.
fn download(target: &str, rev: &str) -> Result<PathBuf, String> {
    if rev.len() < 12 {
        return Err(format!("elegance-rev holds `{rev}`, not a full commit hash"));
    }
    let short = &rev[..12];
    let dir = crate::cmake::cache_dir()
        .ok_or("no user cache folder to put the prebuilt Elegance in")?
        .join("elegance")
        .join("c")
        .join(short)
        .join(target);
    let file = format!("elegance-c-{target}.tar.gz");
    let base = env::var("ELEGANCE_PREBUILT_URL")
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| format!("https://github.com/EleganceUI/elegance/releases/download/c-{short}"));
    fetch(&dir, &format!("{}/{file}", base.trim_end_matches('/')), rev)
}

/// `url`'s archive, extracted into `dir` (unless it's there already) if it was built from `rev`.
fn fetch(dir: &Path, url: &str, rev: &str) -> Result<PathBuf, String> {
    let dir = dir.to_owned();
    if built_from(&dir, rev) {
        return Ok(dir);
    }
    let partial = dir.with_extension("partial");
    let _ = fs::remove_dir_all(&partial);
    fs::create_dir_all(&partial).map_err(|e| format!("can't create {}: {e}", partial.display()))?;
    let archive = partial.join("archive.tar.gz");
    run(
        Command::new(system_tool("curl")).args(["-fsSL", "--retry", "3", "-o"]).arg(&archive).arg(url),
        &format!("downloading {url} (Elegance's CI may not have published this commit's library yet)"),
    )?;
    run(
        Command::new(system_tool("tar")).arg("-xzf").arg(&archive).arg("-C").arg(&partial),
        &format!("extracting {}", archive.display()),
    )?;
    let _ = fs::remove_file(&archive);
    if !built_from(&partial, rev) {
        let _ = fs::remove_dir_all(&partial);
        return Err(format!("the prebuilt Elegance at {url} wasn't built from the commit elegance-rev pins ({rev})"));
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

/// Whether `dir` holds an extracted archive built from Elegance commit `rev` (its BUILDINFO says).
fn built_from(dir: &Path, rev: &str) -> bool {
    fs::read_to_string(dir.join("BUILDINFO"))
        .is_ok_and(|text| text.lines().any(|line| line.trim() == format!("elegance {rev}")))
        && dir.join("lib").is_dir()
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
    ["elegance_c.lib", "libelegance_c.so", "libelegance_c.dylib"]
        .iter()
        .map(|file| lib.join(file))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("\n\nelegance-sys: no elegance_c library in {}.\n", lib.display()))
}
