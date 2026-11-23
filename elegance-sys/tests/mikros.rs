// Run without a Mikros toolchain, from the workspace root:
// rustc --edition=2024 --test elegance-sys/tests/mikros.rs -o target/mikros-tests
#![allow(dead_code)]

#[path = "../build_mikros.rs"]
mod mikros;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

fn lines(vars: &[(&str, &str)], dynamic: bool) -> Result<Vec<String>, String> {
    let vars: HashMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    mikros::link_lines(|name| vars.get(name).cloned(), dynamic)
}

#[test]
fn links_the_system_library_by_its_file_name() {
    let lines = lines(&[], false).unwrap();
    assert!(lines.contains(&"cargo:rustc-link-lib=dylib:+verbatim=eleganceui.0.dlib".to_owned()));
    assert!(!lines.iter().any(|line| line.starts_with("cargo:rustc-link-lib=static")));
    assert!(!lines.iter().any(|line| line.starts_with("cargo:rustc-link-search")));
}

#[test]
fn empty_settings_are_unset() {
    assert!(lines(&[("ELEGANCE_ROOT", ""), ("ELEGANCE_PREBUILT", "")], false).is_ok());
}

#[test]
fn refuses_another_elegance() {
    for name in [
        "ELEGANCE_ROOT",
        "ELEGANCE_GIT_REF",
        "ELEGANCE_BUILD_DIR",
        "ELEGANCE_SHARED_BUILD_DIR",
        "ELEGANCE_NATIVE_DIR",
        "ELEGANCE_PREBUILT",
        "ELEGANCE_PREBUILT_URL",
    ] {
        let error = lines(&[(name, "1")], false).unwrap_err();
        assert!(error.contains("apps on Mikros use the system's library"), "{name}: {error}");
        assert!(error.contains(name), "{error}");
    }
}

#[test]
fn refuses_the_dynamic_feature() {
    let error = lines(&[], true).unwrap_err();
    assert!(error.contains("apps on Mikros use the system's library"), "{error}");
    assert!(error.contains("`dynamic`"), "{error}");
}

/// The major of `project(Elegance VERSION <major>.…)`.
fn project_major(cmake: &str) -> Option<u32> {
    let rest = cmake.split("project(Elegance").nth(1)?;
    let version = rest.split("VERSION").nth(1)?.split_whitespace().next()?;
    version.split('.').next()?.parse().ok()
}

#[test]
fn the_major_is_the_pinned_commits() {
    // An Elegance checkout that has the pinned commit; without one there's nothing to compare.
    // Cargo runs tests in the crate's folder; rustc alone, from the workspace root.
    let sys = option_env!("CARGO_MANIFEST_DIR")
        .map(Path::new)
        .unwrap_or_else(|| Path::new(file!()).parent().unwrap().parent().unwrap());
    let pin = std::fs::read_to_string(sys.join("elegance-rev")).unwrap();
    let roots = std::env::var_os("ELEGANCE_ROOT")
        .map(PathBuf::from)
        .into_iter()
        .chain([sys.join("../../elegance")]);
    for root in roots {
        let Ok(output) = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(["show", &format!("{}:CMakeLists.txt", pin.trim())])
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let cmake = String::from_utf8_lossy(&output.stdout);
        assert_eq!(project_major(&cmake), Some(mikros::ELEGANCE_VERSION_MAJOR), "{}", root.display());
        return;
    }
    eprintln!("no Elegance checkout with the pinned commit: the major isn't compared");
}

#[test]
fn reads_the_project_major() {
    assert_eq!(project_major("cmake_minimum_required(VERSION 3.28)\nproject(Elegance VERSION 0.1.0 LANGUAGES CXX)"), Some(0));
    assert_eq!(project_major("project(Elegance VERSION 12.3)"), Some(12));
}
