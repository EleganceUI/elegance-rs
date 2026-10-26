//! `cargo elegance`: shows an Elegance app's previews and reloads them as you edit.
//!
//! ```text
//! cargo elegance preview            the app's #[preview]s
//! cargo elegance preview --root     its #[preview(root)] view: the whole app
//! cargo elegance build-module       builds the views module, prints its path (what the host reruns)
//!
//!   -p, --package <name>   --bin <name>   --manifest-path <file>   --release
//!   --features <list>      --watch <folder> (more folders to watch than src/ and assets/)
//! ```
//!
//! The preview host (`elegance-preview`) loads an app's views from a DLL, a views module. Rather
//! than making the app a library plus a binary, this builds the app's own binary target as that
//! DLL: it runs `cargo build` with itself as `RUSTC_WORKSPACE_WRAPPER`, which compiles just that
//! crate with `--crate-type cdylib` instead of `bin`, and turns on elegance's `preview` feature
//! (which exports the module entry, registers the #[preview]s, and links elegance.dll, which the
//! host shares). Everything else (dependencies, build scripts, features, `CARGO_MANIFEST_DIR`,
//! `asset!`) is the ordinary build's. It uses its own target directory, `target/elegance-preview`,
//! so the app's own builds aren't disturbed.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::Value;

// Set on the `cargo build` that cargo-elegance runs, for its wrapper mode.
const WRAP_CRATE: &str = "ELEGANCE_PREVIEW_CRATE"; // the crate to build as a module
const WRAP_KIND: &str = "ELEGANCE_PREVIEW_KIND"; // "bin" or "lib"
const WRAP_RECORD: &str = "ELEGANCE_PREVIEW_RECORD"; // where to write the module's path

const USAGE: &str = "\
usage: cargo elegance preview [--root] [options]
       cargo elegance build-module [options]

options: -p/--package <name>  --bin <name>  --manifest-path <file>  --release
         --features <list>  --watch <folder>...";

fn main() -> ExitCode {
    let args: Vec<OsString> = env::args_os().collect();
    // As cargo's RUSTC_WORKSPACE_WRAPPER: `cargo-elegance <rustc> <args>...`.
    if let (Some(name), Some(first)) = (env::var_os(WRAP_CRATE), args.get(1))
        && first != "elegance" {
            return wrap_rustc(&name.to_string_lossy(), &args[1..]);
        }
    // `cargo elegance ...` runs `cargo-elegance elegance ...`.
    let mut args: Vec<String> = args.iter().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    if args.first().is_some_and(|a| a == "elegance") {
        args.remove(0);
    }
    match run(args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(mut args: Vec<String>) -> Result<ExitCode, String> {
    if args.is_empty() {
        return Err(USAGE.to_owned());
    }
    let command = args.remove(0);
    let options = Options::parse(args)?;
    match command.as_str() {
        "preview" => preview(&options),
        "build-module" => {
            let target = Target::find(&options)?;
            println!("{}", build_module(&target, &options)?.display());
            Ok(ExitCode::SUCCESS)
        }
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            Ok(ExitCode::SUCCESS)
        }
        other => Err(format!("unknown command `{other}`\n{USAGE}")),
    }
}

#[derive(Default)]
struct Options {
    root: bool,
    package: Option<String>,
    bin: Option<String>,
    manifest_path: Option<PathBuf>,
    release: bool,
    features: Vec<String>,
    watch: Vec<PathBuf>,
}

impl Options {
    fn parse(args: Vec<String>) -> Result<Options, String> {
        let mut options = Options::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            // --name value, or --name=value.
            let (name, inline) = match arg.split_once('=') {
                Some((name, value)) if name.starts_with("--") => (name.to_owned(), Some(value.to_owned())),
                _ => (arg.clone(), None),
            };
            let mut value = || inline.clone().or_else(|| args.next()).ok_or_else(|| format!("{name} needs a value"));
            match name.as_str() {
                "--root" => options.root = true,
                "--release" => options.release = true,
                "-p" | "--package" => options.package = Some(value()?),
                "--bin" => options.bin = Some(value()?),
                "--manifest-path" => options.manifest_path = Some(PathBuf::from(value()?)),
                "-F" | "--features" => options.features.push(value()?),
                "--watch" => options.watch.push(PathBuf::from(value()?)),
                _ => return Err(format!("unknown option `{arg}`\n{USAGE}")),
            }
        }
        Ok(options)
    }

    // The same options again, for the host's build command.
    fn build_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if let Some(package) = &self.package {
            args.extend(["--package".to_owned(), package.clone()]);
        }
        if let Some(bin) = &self.bin {
            args.extend(["--bin".to_owned(), bin.clone()]);
        }
        if let Some(manifest) = &self.manifest_path {
            args.extend(["--manifest-path".to_owned(), absolute(manifest).display().to_string()]);
        }
        if self.release {
            args.push("--release".to_owned());
        }
        for features in &self.features {
            args.extend(["--features".to_owned(), features.clone()]);
        }
        args
    }
}

/// What to build as the module: a package's binary target (or its library, if it has no binary).
struct Target {
    package: String,
    manifest: PathBuf,
    /// The bin target's name, or None for the library.
    bin: Option<String>,
    /// The crate name rustc sees.
    crate_name: String,
    /// What the package calls the elegance dependency.
    elegance: String,
    target_dir: PathBuf,
}

impl Target {
    fn find(options: &Options) -> Result<Target, String> {
        let mut command = cargo();
        command.args(["metadata", "--format-version", "1", "--no-deps"]);
        if let Some(manifest) = &options.manifest_path {
            command.arg("--manifest-path").arg(manifest);
        }
        let output = command.output().map_err(|e| format!("couldn't run cargo: {e}"))?;
        if !output.status.success() {
            return Err(format!("cargo metadata failed:\n{}", String::from_utf8_lossy(&output.stderr)));
        }
        let metadata: Value =
            serde_json::from_slice(&output.stdout).map_err(|e| format!("couldn't read cargo metadata: {e}"))?;
        let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
        let package = choose_package(&packages, options)?;
        let name = package["name"].as_str().unwrap_or_default().to_owned();
        let manifest = PathBuf::from(package["manifest_path"].as_str().unwrap_or_default());

        let targets = package["targets"].as_array().cloned().unwrap_or_default();
        let has_kind = |t: &Value, kind: &str| t["kind"].as_array().is_some_and(|k| k.iter().any(|k| k == kind));
        let bins: Vec<String> =
            targets.iter().filter(|t| has_kind(t, "bin")).filter_map(|t| t["name"].as_str().map(str::to_owned)).collect();
        let lib = targets.iter().find(|t| ["lib", "rlib", "cdylib", "dylib"].iter().any(|k| has_kind(t, k)));
        let (bin, crate_name) = match &options.bin {
            Some(bin) if bins.contains(bin) => (Some(bin.clone()), bin.replace('-', "_")),
            Some(bin) => return Err(format!("package `{name}` has no binary `{bin}` (it has: {})", bins.join(", "))),
            None if bins.len() == 1 => (Some(bins[0].clone()), bins[0].replace('-', "_")),
            None if bins.contains(&name) => (Some(name.clone()), name.replace('-', "_")),
            None if bins.is_empty() && lib.is_some() => {
                (None, lib.unwrap()["name"].as_str().unwrap_or_default().replace('-', "_"))
            }
            None if bins.is_empty() => return Err(format!("package `{name}` has no binary or library to preview")),
            None => return Err(format!("package `{name}` has several binaries; pick one with --bin ({})", bins.join(", "))),
        };

        let elegance = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|d| d["name"] == "elegance" && d["kind"].is_null())
            .map(|d| d["rename"].as_str().unwrap_or("elegance").to_owned())
            .ok_or_else(|| format!("package `{name}` doesn't depend on elegance"))?;

        let target_dir = PathBuf::from(metadata["target_directory"].as_str().unwrap_or("target")).join("elegance-preview");
        Ok(Target { package: name, manifest, bin, crate_name, elegance, target_dir })
    }

    fn dir(&self) -> &Path {
        self.manifest.parent().unwrap()
    }

    fn profile_dir(&self, options: &Options) -> PathBuf {
        self.target_dir.join(if options.release { "release" } else { "debug" })
    }

    // Where the host loads the module from: named after the app, so its window title is too.
    fn module_path(&self) -> PathBuf {
        let name = self.bin.as_deref().unwrap_or(&self.package);
        self.target_dir.join(format!("{name}{}", env::consts::DLL_SUFFIX))
    }
}

// The package named by -p, or the one whose folder holds the current directory, or the only one.
fn choose_package(packages: &[Value], options: &Options) -> Result<Value, String> {
    let names = || packages.iter().filter_map(|p| p["name"].as_str()).collect::<Vec<_>>().join(", ");
    if let Some(wanted) = &options.package {
        return packages
            .iter()
            .find(|p| p["name"] == wanted.as_str())
            .cloned()
            .ok_or_else(|| format!("no package `{wanted}` in this workspace ({})", names()));
    }
    if let [only] = packages {
        return Ok(only.clone());
    }
    let cwd = env::current_dir().map_err(|e| e.to_string())?;
    packages
        .iter()
        .filter_map(|p| {
            let dir = Path::new(p["manifest_path"].as_str()?).parent()?;
            cwd.starts_with(dir).then(|| (dir.components().count(), p))
        })
        .max_by_key(|(depth, _)| *depth)
        .map(|(_, p)| p.clone())
        .ok_or_else(|| format!("pick a package with -p ({})", names()))
}

// Builds the module and copies it to `Target::module_path` when it changed. Returns that path.
fn build_module(target: &Target, options: &Options) -> Result<PathBuf, String> {
    fs::create_dir_all(&target.target_dir).map_err(|e| e.to_string())?;
    let record = target.target_dir.join(format!("{}.module-path", target.crate_name));
    let self_exe = env::current_exe().map_err(|e| e.to_string())?;

    let mut command = cargo();
    command.arg("build").arg("--manifest-path").arg(&target.manifest).args(["--package", &target.package]);
    match &target.bin {
        Some(bin) => command.args(["--bin", bin]),
        None => command.arg("--lib"),
    };
    command.arg("--features").arg(format!("{}/preview", target.elegance));
    for features in &options.features {
        command.arg("--features").arg(features);
    }
    if options.release {
        command.arg("--release");
    }
    command
        .arg("--target-dir")
        .arg(&target.target_dir)
        .env("RUSTC_WORKSPACE_WRAPPER", &self_exe)
        .env(WRAP_CRATE, &target.crate_name)
        .env(WRAP_KIND, if target.bin.is_some() { "bin" } else { "lib" })
        .env(WRAP_RECORD, &record);
    let status = command.status().map_err(|e| format!("couldn't run cargo: {e}"))?;
    if !status.success() {
        return Err("the build failed".to_owned());
    }

    let built = fs::read_to_string(&record)
        .map(|path| PathBuf::from(path.trim()))
        .map_err(|_| format!("the build didn't say where it put {}'s module", target.crate_name))?;
    let module = target.module_path();
    let changed = match (modified(&built), modified(&module)) {
        (Some(built), Some(copied)) => built > copied,
        (Some(_), None) => true,
        (None, _) => return Err(format!("{} wasn't built", built.display())),
    };
    if changed {
        // Not loaded in place (the host loads a copy), so this never fights a running preview.
        fs::copy(&built, &module).map_err(|e| format!("couldn't copy {}: {e}", built.display()))?;
    }
    Ok(module)
}

fn preview(options: &Options) -> Result<ExitCode, String> {
    let target = Target::find(options)?;
    let module = build_module(&target, options)?;

    // elegance-sys copies the host from the shared Elegance build, beside elegance.dll.
    let host = target.profile_dir(options).join(format!("elegance-preview{}", env::consts::EXE_SUFFIX));
    if !host.exists() {
        return Err(format!(
            "{} not found. elegance-sys copies it from the shared Elegance build it links: build \
             elegance-preview there\n(cmake --build build/dev-release --target elegance-preview), or unset \
             ELEGANCE_SHARED_BUILD_DIR to let elegance-sys build Elegance itself.",
            host.display()
        ));
    }

    let self_exe = env::current_exe().map_err(|e| e.to_string())?;
    let mut build = vec![self_exe.display().to_string(), "elegance".to_owned(), "build-module".to_owned()];
    build.extend(options.build_args());
    let build = build.iter().map(|a| shell_quote(a)).collect::<Vec<_>>().join(" ");

    let mut command = Command::new(&host);
    if options.root {
        command.arg("--root");
    }
    command.arg("--module").arg(&module).arg("--build").arg(&build);
    let mut watch: Vec<PathBuf> =
        ["src", "assets"].iter().map(|d| target.dir().join(d)).filter(|d| d.is_dir()).collect();
    watch.extend(options.watch.iter().map(|d| absolute(d)));
    for folder in &watch {
        command.arg("--watch").arg(folder);
    }
    println!("     Preview {} (watching {})", target.package, watch.iter().map(|w| w.display().to_string()).collect::<Vec<_>>().join(", "));
    let status = command.current_dir(target.dir()).status().map_err(|e| format!("couldn't start {}: {e}", host.display()))?;
    Ok(if status.success() { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

// Wrapper mode: runs rustc, building the module's crate as a cdylib instead of a bin (or lib).
fn wrap_rustc(crate_name: &str, args: &[OsString]) -> ExitCode {
    let rustc = &args[0];
    let mut args: Vec<OsString> = args[1..].to_vec();
    let text: Vec<String> = args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    let pair = |flag: &str| text.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone());
    let kinds: Vec<&str> = text.windows(2).filter(|w| w[0] == "--crate-type").map(|w| w[1].as_str()).collect();
    let wanted = env::var(WRAP_KIND).unwrap_or_default();
    let is_module = pair("--crate-name").as_deref() == Some(crate_name)
        && match wanted.as_str() {
            "bin" => kinds == ["bin"],
            _ => !kinds.is_empty() && kinds.iter().all(|k| ["lib", "rlib", "dylib", "cdylib", "staticlib"].contains(k)),
        };

    if is_module {
        // One `--crate-type cdylib` in place of cargo's crate types.
        let mut rewritten = Vec::with_capacity(args.len());
        let mut typed = false;
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--crate-type" && i + 1 < args.len() {
                if !typed {
                    rewritten.extend([OsString::from("--crate-type"), OsString::from("cdylib")]);
                    typed = true;
                }
                i += 2;
            } else {
                rewritten.push(args[i].clone());
                i += 1;
            }
        }
        args = rewritten;
        if wanted == "bin" {
            args.extend([OsString::from("-A"), OsString::from("dead_code")]); // fn main, as a library
        }
    }

    let status = match Command::new(rustc).args(&args).status() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("cargo-elegance: couldn't run {}: {e}", rustc.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    if is_module && status.success() {
        // Where rustc put it: <out-dir>/<prefix><crate><extra-filename><suffix>.
        let extra = text
            .iter()
            .zip(text.iter().skip(1))
            .find_map(|(a, b)| {
                let option = if a == "-C" { b.as_str() } else { a.strip_prefix("-C")? };
                option.strip_prefix("extra-filename=")
            })
            .unwrap_or("");
        if let (Some(out_dir), Some(record)) = (pair("--out-dir"), env::var_os(WRAP_RECORD)) {
            let file = format!("{}{crate_name}{extra}{}", env::consts::DLL_PREFIX, env::consts::DLL_SUFFIX);
            let _ = fs::write(record, Path::new(&out_dir).join(file).display().to_string());
        }
    }
    ExitCode::from(status.code().unwrap_or(1).clamp(0, 255) as u8)
}

fn cargo() -> Command {
    Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

// The host runs its build command through the shell.
fn shell_quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"', '&', '|', '<', '>', '^', '\'']) {
        arg.to_owned()
    } else if cfg!(windows) {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}
