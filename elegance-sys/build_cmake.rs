// Builds Elegance with CMake when there's no prebuilt tree to link against.
//
// Elegance's dependencies come from vcpkg (manifest mode, pinned by vcpkg.json's builtin-baseline).
// The vcpkg used is the first that has that baseline:
//
//   1. VCPKG_ROOT
//   2. the VCPKG_ROOT that Elegance's CMakeUserPresets.json / CMakePresets.json set
//   3. `vcpkg` on PATH
//   4. a clone this script makes and bootstraps, in ELEGANCE_VCPKG_DIR (default: the user cache
//      folder, %LOCALAPPDATA%\elegance\vcpkg or ~/.cache/elegance/vcpkg)
//
// CMake (the version Elegance requires) and Ninja come from CMAKE / NINJA, PATH, Visual Studio's
// bundled copies, or `vcpkg fetch`. On MSVC targets the compiler environment comes from the cc
// crate's Visual Studio lookup, so no developer prompt is needed.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const VCPKG_REPO: &str = "https://github.com/microsoft/vcpkg";
const ELEGANCE_REPO: &str = "https://github.com/EleganceUI/elegance";

/// Elegance's source, cloned once into the user cache folder (one folder per branch or tag), for
/// builds with no checkout of their own. Delete the folder to get a newer one.
pub fn clone_elegance(reference: &str) -> PathBuf {
    let name: String =
        reference.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    let dir = env::var_os("ELEGANCE_CACHE_DIR")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| cache_dir().map(|cache| cache.join("elegance")))
        .unwrap_or_else(|| panic!("\n\nelegance-sys: no Elegance source found; set ELEGANCE_ROOT.\n"))
        .join("src")
        .join(name);
    if !dir.join("CMakeLists.txt").is_file() {
        println!("cargo:warning=cloning Elegance ({reference}) into {}", dir.display());
        let partial = dir.with_extension("partial");
        let _ = fs::remove_dir_all(&partial);
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        let args = ["clone", "--depth", "1", "--branch", reference, ELEGANCE_REPO];
        let mut args: Vec<OsString> = args.iter().map(OsString::from).collect();
        args.push(partial.clone().into());
        git(None, &args);
        fs::rename(&partial, &dir)
            .unwrap_or_else(|e| panic!("elegance-sys: couldn't move Elegance to {}: {e}", dir.display()));
    }
    dir
}

/// Configures (once) and builds Elegance in `build`: static, or shared plus the preview host.
pub fn build_elegance(root: &Path, build: &Path, shared: bool) {
    for watched in ["src", "include", "cmake", "apps/preview", "CMakeLists.txt", "vcpkg.json"] {
        println!("cargo:rerun-if-changed={}", root.join(watched).display());
    }
    for var in ["VCPKG_ROOT", "ELEGANCE_VCPKG_DIR", "ELEGANCE_CACHE_DIR", "CMAKE", "NINJA"] {
        println!("cargo:rerun-if-env-changed={var}");
    }

    let msvc_env = msvc_env();
    let vcpkg = find_vcpkg(root);
    let cmake = find_cmake(root, &vcpkg);
    let ninja = find_ninja(&vcpkg);

    let run = |args: &[OsString], what: &str| {
        let mut command = Command::new(&cmake);
        command.args(args).env("VCPKG_ROOT", &vcpkg.root);
        for (key, value) in &msvc_env {
            command.env(key, value);
        }
        // A build script's stdout is for cargo: send CMake's output to stderr.
        command.stdout(Stdio::from(io::stderr()));
        let status = command.status().unwrap_or_else(|e| panic!("elegance-sys: couldn't run {}: {e}", cmake.display()));
        if !status.success() {
            panic!(
                "\n\nelegance-sys: {what} Elegance failed (in {}); CMake's output is above. Run cargo with -vv to \
                 see it as it happens.\n",
                build.display()
            );
        }
    };

    if !build.join("CMakeCache.txt").exists() {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        let mut args: Vec<OsString> = vec![
            "-S".into(),
            root.into(),
            "-B".into(),
            build.into(),
            "-G".into(),
            "Ninja".into(),
            define("CMAKE_MAKE_PROGRAM", ninja.as_os_str()),
            define("CMAKE_TOOLCHAIN_FILE", vcpkg.root.join("scripts/buildsystems/vcpkg.cmake").as_os_str()),
            // The release C runtime (/MD), which Rust links.
            define("CMAKE_BUILD_TYPE", "RelWithDebInfo"),
            define("VCPKG_MANIFEST_FEATURES", ""),
            define("ELG_BUILD_SHARED", on(shared)),
            // AccessKit's static library is a Rust staticlib, whose Rust std clashes with the app's.
            define("ELG_ACCESSKIT_SHARED", "ON"),
            define("ELG_BUILD_TESTS", "OFF"),
            // The sandbox option also brings the preview host, which a shared build is for.
            define("ELG_BUILD_SANDBOX", on(shared)),
        ];
        if let Some(triplet) = vcpkg_triplet() {
            args.push(define("VCPKG_TARGET_TRIPLET", triplet));
        }
        if !msvc_env.is_empty() {
            args.push(define("CMAKE_CXX_COMPILER", "cl"));
        }
        println!("cargo:warning=building Elegance in {} (the first build takes a while)", build.display());
        run(&args, "configuring");
    }

    let mut args: Vec<OsString> = vec!["--build".into(), build.into(), "--target".into(), "elegance".into()];
    if shared {
        args.push("elegance-preview".into());
    }
    run(&args, "building");
}

fn define(name: &str, value: impl Into<OsString>) -> OsString {
    let mut arg = OsString::from(format!("-D{name}="));
    arg.push(value.into());
    arg
}

fn target() -> String {
    env::var("TARGET").unwrap()
}

fn vcpkg_triplet() -> Option<&'static str> {
    let target = target();
    let arch = if target.starts_with("x86_64") {
        "x64"
    } else if target.starts_with("aarch64") {
        "arm64"
    } else {
        return None;
    };
    let os = if target.contains("windows") {
        "windows"
    } else if target.contains("apple") {
        "osx"
    } else if target.contains("linux") {
        "linux"
    } else {
        return None;
    };
    Some(match (arch, os) {
        ("x64", "windows") => "x64-windows",
        ("arm64", "windows") => "arm64-windows",
        ("x64", "osx") => "x64-osx",
        ("arm64", "osx") => "arm64-osx",
        ("x64", "linux") => "x64-linux",
        _ => "arm64-linux",
    })
}

// PATH, INCLUDE, LIB, ... for MSVC's compiler, from cc's Visual Studio lookup (empty elsewhere).
fn msvc_env() -> Vec<(OsString, OsString)> {
    let target = target();
    if !target.contains("msvc") {
        return Vec::new();
    }
    let tool = cc::windows_registry::find_tool(&target, "cl.exe").unwrap_or_else(|| {
        panic!("\n\nelegance-sys: couldn't find Visual Studio's C++ compiler (cl.exe) to build Elegance with.\n")
    });
    tool.get_envs().map(|(k, v)| (k.to_owned(), v.to_owned())).collect()
}

struct Vcpkg {
    root: PathBuf,
}

impl Vcpkg {
    fn exe(&self) -> PathBuf {
        self.root.join(if cfg!(windows) { "vcpkg.exe" } else { "vcpkg" })
    }

    /// A tool vcpkg downloads (and caches) itself: `vcpkg fetch cmake` prints its path.
    fn fetch(&self, tool: &str) -> Option<PathBuf> {
        let output = Command::new(self.exe()).args(["fetch", tool]).stderr(Stdio::inherit()).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(stdout.lines().rev().find(|l| !l.trim().is_empty())?.trim());
        path.is_file().then_some(path)
    }
}

fn find_vcpkg(root: &Path) -> Vcpkg {
    let baseline = baseline(root);
    let usable = |dir: &Path| {
        dir.join("scripts/buildsystems/vcpkg.cmake").is_file()
            && Vcpkg { root: dir.to_owned() }.exe().is_file()
            && baseline.as_deref().is_none_or(|commit| has_commit(dir, commit))
    };

    // VCPKG_ROOT is the user's choice: use it even without the baseline (vcpkg says what's wrong).
    if let Some(dir) = env::var_os("VCPKG_ROOT").filter(|d| !d.is_empty()).map(PathBuf::from) {
        if dir.join("scripts/buildsystems/vcpkg.cmake").is_file() {
            return Vcpkg { root: dir };
        }
        println!("cargo:warning=VCPKG_ROOT ({}) isn't a vcpkg checkout; looking elsewhere", dir.display());
    }
    let from_presets = ["CMakeUserPresets.json", "CMakePresets.json"]
        .iter()
        .filter_map(|file| fs::read_to_string(root.join(file)).ok())
        .flat_map(|text| preset_vcpkg_roots(&text));
    let on_path = find_on_path("vcpkg").and_then(|exe| Some(exe.parent()?.to_owned()));
    if let Some(dir) = from_presets.chain(on_path).find(|dir| usable(dir)) {
        return Vcpkg { root: dir };
    }

    let dir = env::var_os("ELEGANCE_VCPKG_DIR")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| cache_dir().map(|cache| cache.join("elegance").join("vcpkg")))
        .unwrap_or_else(|| panic!("\n\nelegance-sys: no vcpkg found; set VCPKG_ROOT or ELEGANCE_VCPKG_DIR.\n"));
    bootstrap_vcpkg(&dir, baseline.as_deref());
    Vcpkg { root: dir }
}

// Our own vcpkg: clone it (the whole history, so baselines resolve), update it if it predates the
// baseline, and build vcpkg's executable.
fn bootstrap_vcpkg(dir: &Path, baseline: Option<&str>) {
    if !dir.join(".git").exists() {
        println!("cargo:warning=cloning vcpkg into {}", dir.display());
        let partial = dir.with_extension("partial");
        let _ = fs::remove_dir_all(&partial);
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        git(None, &[OsString::from("clone"), VCPKG_REPO.into(), partial.clone().into()]);
        fs::rename(&partial, dir)
            .unwrap_or_else(|e| panic!("elegance-sys: couldn't move vcpkg to {}: {e}", dir.display()));
    } else if baseline.is_some_and(|commit| !has_commit(dir, commit)) {
        git(Some(dir), &["fetch".into(), "origin".into()]);
    }
    let vcpkg = Vcpkg { root: dir.to_owned() };
    if !vcpkg.exe().is_file() {
        let script = if cfg!(windows) { dir.join("bootstrap-vcpkg.bat") } else { dir.join("bootstrap-vcpkg.sh") };
        let status = Command::new(&script)
            .arg("-disableMetrics")
            .current_dir(dir)
            .stdout(Stdio::from(io::stderr()))
            .status()
            .unwrap_or_else(|e| panic!("elegance-sys: couldn't run {}: {e}", script.display()));
        if !status.success() || !vcpkg.exe().is_file() {
            panic!("\n\nelegance-sys: bootstrapping vcpkg in {} failed.\n", dir.display());
        }
    }
}

fn git(dir: Option<&Path>, args: &[OsString]) {
    let mut command = Command::new("git");
    if let Some(dir) = dir {
        command.arg("-C").arg(dir);
    }
    let status = command.args(args).stdout(Stdio::from(io::stderr())).status().unwrap_or_else(|e| {
        panic!("\n\nelegance-sys: git is needed to get vcpkg ({e}); install it or set VCPKG_ROOT.\n")
    });
    if !status.success() {
        panic!("\n\nelegance-sys: `git {}` failed.\n", args.join(" ".as_ref()).to_string_lossy());
    }
}

fn has_commit(dir: &Path, commit: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["cat-file", "-e", &format!("{commit}^{{commit}}")])
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

// vcpkg.json's "builtin-baseline".
fn baseline(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join("vcpkg.json")).ok()?;
    json_string_after(&text, "\"builtin-baseline\"").into_iter().next()
}

// Every `"VCPKG_ROOT": "<path>"` in a presets file.
fn preset_vcpkg_roots(text: &str) -> Vec<PathBuf> {
    json_string_after(text, "\"VCPKG_ROOT\"").into_iter().filter(|v| !v.contains('$')).map(PathBuf::from).collect()
}

// The string values that follow `key:` (enough JSON for vcpkg.json and presets files).
fn json_string_after(text: &str, key: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(key) {
        rest = &rest[at + key.len()..];
        let Some(value) = rest.trim_start().strip_prefix(':') else {
            continue;
        };
        let Some(value) = value.trim_start().strip_prefix('"') else {
            continue;
        };
        let Some(end) = value.find('"') else { break };
        values.push(value[..end].replace("\\\\", "\\"));
    }
    values
}

pub fn cache_dir() -> Option<PathBuf> {
    let var = |name: &str| env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    if cfg!(windows) {
        var("LOCALAPPDATA")
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library").join("Caches"))
    } else {
        var("XDG_CACHE_HOME").or_else(|| var("HOME").map(|home| home.join(".cache")))
    }
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let file = if cfg!(windows) { format!("{name}.exe") } else { name.to_owned() };
    env::split_paths(&env::var_os("PATH")?).map(|dir| dir.join(&file)).find(|p| p.is_file())
}

// Visual Studio's bundled CMake and Ninja (Common7/IDE/CommonExtensions/Microsoft/CMake/...).
fn visual_studio_cmake_dir() -> Option<PathBuf> {
    let vswhere = PathBuf::from(env::var_os("ProgramFiles(x86)")?)
        .join("Microsoft Visual Studio")
        .join("Installer")
        .join("vswhere.exe");
    let output =
        Command::new(vswhere).args(["-latest", "-products", "*", "-property", "installationPath"]).output().ok()?;
    let install = String::from_utf8_lossy(&output.stdout).lines().next()?.trim().to_owned();
    let dir = ["Common7", "IDE", "CommonExtensions", "Microsoft", "CMake"]
        .iter()
        .fold(PathBuf::from(install), |d, c| d.join(c));
    dir.is_dir().then_some(dir)
}

fn find_cmake(root: &Path, vcpkg: &Vcpkg) -> PathBuf {
    let required = required_cmake(root);
    let new_enough = |exe: &Path| cmake_version(exe).is_some_and(|v| v >= required);
    let exe = if cfg!(windows) { "cmake.exe" } else { "cmake" };
    let candidates = env::var_os("CMAKE")
        .map(PathBuf::from)
        .into_iter()
        .chain(find_on_path("cmake"))
        .chain(visual_studio_cmake_dir().map(|dir| dir.join("CMake").join("bin").join(exe)));
    let found = candidates.filter(|exe| exe.is_file()).find(|exe| new_enough(exe));
    found.or_else(|| vcpkg.fetch("cmake").filter(|exe| new_enough(exe))).unwrap_or_else(|| {
        let (major, minor) = required;
        panic!("\n\nelegance-sys: Elegance needs CMake {major}.{minor} or newer; install it or set CMAKE to one.\n")
    })
}

fn find_ninja(vcpkg: &Vcpkg) -> PathBuf {
    let exe = if cfg!(windows) { "ninja.exe" } else { "ninja" };
    env::var_os("NINJA")
        .map(PathBuf::from)
        .into_iter()
        .chain(find_on_path("ninja"))
        .chain(visual_studio_cmake_dir().map(|dir| dir.join("Ninja").join(exe)))
        .find(|exe| exe.is_file())
        .or_else(|| vcpkg.fetch("ninja"))
        .unwrap_or_else(|| panic!("\n\nelegance-sys: couldn't find or fetch Ninja; install it or set NINJA.\n"))
}

// cmake_minimum_required(VERSION x.y) in Elegance's CMakeLists.txt.
fn required_cmake(root: &Path) -> (u32, u32) {
    let text = fs::read_to_string(root.join("CMakeLists.txt")).unwrap_or_default();
    text.split_once("cmake_minimum_required(VERSION")
        .and_then(|(_, rest)| parse_version(rest.trim_start()))
        .unwrap_or((3, 25))
}

fn cmake_version(exe: &Path) -> Option<(u32, u32)> {
    let output = Command::new(exe).arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_version(stdout.lines().next()?.strip_prefix("cmake version ")?)
}

// "4.2.3-msvc3" -> (4, 2)
fn parse_version(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.split(|c: char| !c.is_ascii_digit());
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}
