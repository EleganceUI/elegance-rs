// Mikros: apps link the system's Elegance, eleganceui.<major>.dlib, dynamically and by name.
// Nothing is downloaded, built or compiled here, and the variables that pick another Elegance
// (ELEGANCE_ROOT, ELEGANCE_BUILD_DIR, a prebuilt package, ...) and the `dynamic` feature are
// refused.
//
// The linker looks for the library in its search folders: the sysroot's, and any given with
// `-L` (for example RUSTFLAGS="-L native=<folder>" with a copy of the library).
//
// The library carries no header, so the bindings aren't compared with one; App::new compares
// the library's ABI version with the one the bindings were generated for.

/// The major of the version of the pinned Elegance commit (elegance-rev): the project() VERSION
/// of its CMakeLists.txt, which names the library. Change it with the pin when that changes.
pub const ELEGANCE_VERSION_MAJOR: u32 = 0;

/// The Mikros build of Elegance's C API: the system's library.
pub fn library_name() -> String {
    format!("eleganceui.{ELEGANCE_VERSION_MAJOR}.dlib")
}

/// Settings that ask for an Elegance other than the system's.
const OTHER_ELEGANCE: &[&str] = &[
    "ELEGANCE_ROOT",
    "ELEGANCE_GIT_REF",
    "ELEGANCE_BUILD_DIR",
    "ELEGANCE_SHARED_BUILD_DIR",
    "ELEGANCE_NATIVE_DIR",
    "ELEGANCE_PREBUILT",
    "ELEGANCE_PREBUILT_URL",
];

/// The build script's output lines for Mikros, or the error to fail with. `var` reads an
/// environment variable; `dynamic` is whether the `dynamic` feature is on.
pub fn link_lines(var: impl Fn(&str) -> Option<String>, dynamic: bool) -> Result<Vec<String>, String> {
    let mut lines: Vec<String> = OTHER_ELEGANCE
        .iter()
        .map(|name| format!("cargo:rerun-if-env-changed={name}"))
        .collect();
    let mut asked = Vec::new();
    if dynamic {
        asked.push("the `dynamic` feature (a shared build of Elegance)".to_owned());
    }
    for name in OTHER_ELEGANCE {
        if let Some(value) = var(name).filter(|value| !value.is_empty()) {
            asked.push(format!("{name}={value}"));
        }
    }
    if !asked.is_empty() {
        return Err(format!(
            "apps on Mikros use the system's library, {}, linked dynamically: elegance-sys can't link Elegance \
             statically or link another build of it there. Remove {}.",
            library_name(),
            asked.join(", ")
        ));
    }
    // Verbatim: the file has no `lib` prefix, and the name is the file's.
    lines.push(format!("cargo:rustc-link-lib=dylib:+verbatim={}", library_name()));
    lines.push("cargo:dynamic=true".to_owned());
    Ok(lines)
}
