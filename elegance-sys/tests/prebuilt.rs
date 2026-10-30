// Run without downloading/building Elegance:
// rustc --edition=2024 --test elegance-sys/tests/prebuilt.rs -o target/prebuilt-tests
#![allow(dead_code)]

mod cmake {
    pub fn cache_dir() -> Option<std::path::PathBuf> {
        None
    }
}
#[path = "../build_prebuilt.rs"]
mod prebuilt;

use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Package(PathBuf);
impl Package {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "elegance-static-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("lib")).unwrap();
        let package = Self(root);
        package.file("lib/libelegance_c.a", "");
        package.file("lib/libelegance.a", "");
        package.manifest("");
        package
    }
    fn file(&self, path: &str, content: &str) {
        fs::write(self.0.join(path), content).unwrap();
    }
    fn manifest(&self, extra: &str) {
        self.file(
            "elegance-link.txt",
            &format!("static lib/libelegance_c.a\nstatic lib/libelegance.a\n{extra}"),
        );
    }
}
impl Drop for Package {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn resolves_static_archives_in_link_order_and_separates_runtime() {
    let package = Package::new();
    package.file("lib/libaccesskit.dylib", "");
    package.manifest("lib lib/libaccesskit.dylib\nruntime lib/libaccesskit.dylib\nlib -framework Foundation\nlib c++\n");
    let link = prebuilt::link_manifest(&package.0).unwrap();
    assert_eq!(
        link.libs.iter().map(|l| l.0).collect::<Vec<_>>(),
        ["static", "static", "lib", "lib", "lib"]
    );
    assert!(link.libs[0].1.ends_with("/lib/libelegance_c.a"));
    assert!(link.libs[1].1.ends_with("/lib/libelegance.a"));
    assert_eq!(link.libs[3].1, "-framework Foundation");
    assert_eq!(link.runtime.len(), 1);
    assert!(link.runtime[0].ends_with("libaccesskit.dylib"));
}

#[test]
fn rejects_old_shared_packages_and_missing_archives() {
    let package = Package::new();
    package.file("elegance-link.txt", "lib elegance_c\n");
    assert!(prebuilt::link_manifest(&package.0).is_err());
    package.file("elegance-link.txt", "static lib/libelegance_c.a\n");
    assert!(prebuilt::link_manifest(&package.0).is_err());
    package.manifest("");
    fs::remove_file(package.0.join("lib/libelegance.a")).unwrap();
    assert!(prebuilt::link_manifest(&package.0).is_err());
}

#[test]
fn rejects_non_relocatable_or_escaping_paths() {
    let package = Package::new();
    for entry in [
        "static /tmp/library.a",
        "static lib/../library.a",
        "lib Boost::context",
        "lib /usr/lib/libc++.dylib",
        "runtime lib/../../escape.dll",
        "static lib\\library.lib",
    ] {
        package.manifest(entry);
        assert!(
            prebuilt::link_manifest(&package.0).is_err(),
            "accepted {entry}"
        );
    }
}

#[test]
fn rejects_shared_elegance_and_accesskit_staticlib() {
    let package = Package::new();
    for name in ["libelegance_c.dylib", "elegance.dll", "libaccesskit.a"] {
        package.file(&format!("lib/{name}"), "");
        let kind = if name.ends_with(".a") {
            "static"
        } else {
            "lib"
        };
        package.manifest(&format!("{kind} lib/{name}"));
        assert!(
            prebuilt::link_manifest(&package.0).is_err(),
            "accepted {name}"
        );
    }
}

#[test]
fn windows_static_archives_are_explicit_not_inferred_from_lib_extension() {
    let package = Package::new();
    package.file("lib/elegance_c.lib", "");
    package.file("lib/elegance.lib", "");
    package.file(
        "elegance-link.txt",
        "static lib/elegance_c.lib\nstatic lib/elegance.lib\nlib user32.lib\n",
    );
    let link = prebuilt::link_manifest(&package.0).unwrap();
    assert_eq!(link.libs[0].0, "static");
    package.file(
        "elegance-link.txt",
        "lib lib/elegance_c.lib\nlib lib/elegance.lib\n",
    );
    assert!(prebuilt::link_manifest(&package.0).is_err());
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A real Rust executable must keep working after all native archives disappear. No
/// Elegance DLL/dylib is created by this fixture, so dynamic linking cannot satisfy it.
#[test]
#[cfg(unix)]
fn rust_executable_contains_prebuilt_code_after_package_is_removed() {
    let package = Package::new();
    package.file(
        "api.c",
        "extern int elegance_value(void); int elg_value(void) { return elegance_value(); }\n",
    );
    package.file("core.c", "int elegance_value(void) { return 42; }\n");
    for (source, name) in [("api", "elegance_c"), ("core", "elegance")] {
        let object = package.0.join(format!("{source}.o"));
        run(Command::new("cc")
            .arg("-c")
            .arg(package.0.join(format!("{source}.c")))
            .arg("-o")
            .arg(&object));
        let archive = package.0.join(format!("lib/lib{name}.a"));
        fs::remove_file(&archive).unwrap();
        run(Command::new("ar").arg("crs").arg(archive).arg(object));
    }
    // Prove that no build-machine absolute paths are needed after relocation.
    let relocated = Package(package.0.with_extension("relocated"));
    fs::rename(&package.0, &relocated.0).unwrap();
    let link = prebuilt::link_manifest(&relocated.0).unwrap();
    relocated.file("bindings.rs", "unsafe extern \"C\" { fn elg_value() -> i32; } pub fn value() -> i32 { unsafe { elg_value() } }\n");
    relocated.file(
        "main.rs",
        "fn main() { assert_eq!(bindings::value(), 42); }\n",
    );
    let executable = relocated.0.join("app");
    let bindings = relocated.0.join("libbindings.rlib");
    let mut rustc = Command::new("rustc");
    rustc
        .args([
            "--edition=2024",
            "--crate-name=bindings",
            "--crate-type=rlib",
        ])
        .arg(relocated.0.join("bindings.rs"))
        .arg("-o")
        .arg(&bindings);
    for (kind, file) in link.libs {
        let file = PathBuf::from(file);
        let name = file
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_start_matches("lib");
        rustc
            .arg("-L")
            .arg(format!("native={}", file.parent().unwrap().display()));
        // Matches build.rs: link static archives at the final executable link, not into
        // elegance-sys's intermediate rlib (static:-bundle).
        rustc.arg("-l").arg(format!("{kind}:-bundle={name}"));
    }
    run(&mut rustc);
    // Just as Cargo does for an application depending on elegance-sys, native linkage
    // must propagate through the rlib to the final executable.
    run(Command::new("rustc")
        .arg("--edition=2024")
        .arg(relocated.0.join("main.rs"))
        .arg("--extern")
        .arg(format!("bindings={}", bindings.display()))
        .arg("-L")
        .arg(format!("native={}", relocated.0.join("lib").display()))
        .arg("-o")
        .arg(&executable));
    fs::remove_dir_all(relocated.0.join("lib")).unwrap();
    run(Command::new(&executable)
        .env_remove("DYLD_LIBRARY_PATH")
        .env_remove("LD_LIBRARY_PATH"));
    let output = if cfg!(target_os = "macos") {
        Command::new("otool")
            .arg("-L")
            .arg(&executable)
            .output()
            .unwrap()
    } else {
        Command::new("ldd").arg(&executable).output().unwrap()
    };
    assert!(output.status.success());
    let dependencies = String::from_utf8_lossy(&output.stdout);
    let dependencies = dependencies
        .lines()
        .skip(usize::from(cfg!(target_os = "macos")));
    assert!(
        dependencies
            .into_iter()
            .all(|line| !line.contains("elegance"))
    );
}
