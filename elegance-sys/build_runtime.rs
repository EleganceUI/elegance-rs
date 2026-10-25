use std::path::{Path, PathBuf};

pub fn shared_library(build: &Path) -> Option<PathBuf> {
    // CMake puts DLL import libraries in lib/, but shared libraries in bin/.
    [
        "lib/elegance.lib",
        "bin/libelegance.so",
        "bin/libelegance.dylib",
        "lib/libelegance.so",
        "lib/libelegance.dylib",
    ]
    .iter()
    .map(|file| build.join(file))
    .find(|file| file.is_file())
}

pub fn is_runtime_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".dll")
        || name.ends_with(".dylib")
        || name.ends_with(".so")
        || name.contains(".so.")
        || name == "elegance-preview"
        || name == "elegance-preview.exe"
}
