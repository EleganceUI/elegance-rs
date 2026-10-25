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

// Cargo does not propagate dependency build scripts' rpath linker arguments to applications.
// Relocate private copies instead: dylib IDs also become the consumers' recorded load paths.
pub fn prepare_macos_runtime(files: &[PathBuf], out_dir: &Path) -> Vec<PathBuf> {
    use std::{collections::BTreeSet, fs, io::Read, process::Command};

    fn run(command: &mut Command) -> std::process::Output {
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("failed to run {command:?}: {error}"));
        assert!(
            output.status.success(),
            "{command:?} failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    let runtime = out_dir.join("runtime");
    fs::create_dir_all(&runtime)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", runtime.display()));
    let staged: Vec<_> = files
        .iter()
        .map(|file| {
            let path = runtime.join(file.file_name().expect("runtime file must have a filename"));
            fs::copy(file, &path).unwrap_or_else(|error| {
                panic!(
                    "failed to copy {} to {}: {error}",
                    file.display(),
                    path.display()
                )
            });
            path
        })
        .collect();
    let dylibs: BTreeSet<_> = staged
        .iter()
        .filter(|path| path.extension().is_some_and(|ext| ext == "dylib"))
        .map(|path| path.file_name().unwrap())
        .collect();

    for path in &staged {
        let mut magic = [0; 4];
        let mut file = fs::File::open(path)
            .unwrap_or_else(|error| panic!("failed to open {}: {error}", path.display()));
        let count = file
            .read(&mut magic)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        // Thin (32/64-bit, either byte order) and universal Mach-O headers.
        if count != 4
            || !matches!(
                u32::from_be_bytes(magic),
                0xfeedface
                    | 0xcefaedfe
                    | 0xfeedfacf
                    | 0xcffaedfe
                    | 0xcafebabe
                    | 0xbebafeca
                    | 0xcafebabf
                    | 0xbfbafeca
            )
        {
            continue;
        }

        let output = run(Command::new("otool").arg("-L").arg(path));
        let dependencies = String::from_utf8(output.stdout)
            .unwrap_or_else(|error| panic!("invalid otool output for {}: {error}", path.display()));
        let mut changes = Command::new("install_name_tool");
        let mut modified = false;
        if path.extension().is_some_and(|ext| ext == "dylib") {
            let name = path
                .file_name()
                .unwrap()
                .to_str()
                .expect("dylib filename must be UTF-8");
            changes.arg("-id").arg(format!("@loader_path/{name}"));
            modified = true;
        }
        // Universal binaries can list the same dependency once per architecture.
        let mut seen = BTreeSet::new();
        for line in dependencies.lines() {
            let Some((dependency, _)) = line.trim().rsplit_once(" (compatibility version ") else {
                continue;
            };
            let dependency_path = Path::new(dependency);
            let Some(name) = dependency_path.file_name() else {
                continue;
            };
            // Only supplied dylibs are relocated, even for absolute system paths.
            if (dependency.starts_with("@rpath/") || dependency_path.is_absolute())
                && dylibs.contains(name)
                && seen.insert(dependency)
            {
                let name = name.to_str().expect("dylib filename must be UTF-8");
                changes
                    .arg("-change")
                    .arg(dependency)
                    .arg(format!("@loader_path/{name}"));
                modified = true;
            }
        }
        if modified {
            run(changes.arg(path));
            run(Command::new("codesign")
                .args(["--force", "--sign", "-"])
                .arg(path));
        }
    }
    staged
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

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn run(command: &mut Command) -> String {
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("{command:?}: {error}"));
        assert!(
            output.status.success(),
            "{command:?}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn staged_macos_runtime_runs_without_originals_or_rpath() {
        let temp = TempDir(std::env::temp_dir().join(format!(
            "elegance-runtime-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        )));
        let source = temp.0.join("original files");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("leaf.c"), "int leaf(void) { return 42; }").unwrap();
        fs::write(
            source.join("middle.c"),
            "extern int leaf(void); int middle(void) { return leaf(); }",
        )
        .unwrap();
        fs::write(
            source.join("preview.c"),
            "extern int middle(void); int main(void) { return middle() == 42 ? 0 : 1; }",
        )
        .unwrap();
        let leaf = source.join("libleaf.dylib");
        let middle = source.join("libmiddle.dylib");
        let preview = source.join("elegance-preview");
        run(Command::new("clang")
            .arg("-dynamiclib")
            .arg(source.join("leaf.c"))
            .args([
                "-Wl,-headerpad_max_install_names",
                "-install_name",
                "@rpath/libleaf.dylib",
            ])
            .arg("-o")
            .arg(&leaf));
        run(Command::new("clang")
            .arg("-dynamiclib")
            .arg(source.join("middle.c"))
            .arg(&leaf)
            .args([
                "-Wl,-headerpad_max_install_names",
                "-install_name",
                "@rpath/libmiddle.dylib",
            ])
            .arg("-o")
            .arg(&middle));
        run(Command::new("clang")
            .arg(source.join("preview.c"))
            .arg(&middle)
            .arg(&leaf)
            .arg("-Wl,-headerpad_max_install_names")
            .arg("-o")
            .arg(&preview));
        let resource = source.join("resource.txt");
        fs::write(&resource, b"unchanged resource\x00\xff").unwrap();
        let files = [middle, resource, preview, leaf];
        let originals: Vec<_> = files.iter().map(|path| fs::read(path).unwrap()).collect();
        let ids: Vec<_> = [&files[0], &files[3]]
            .iter()
            .map(|path| run(Command::new("otool").arg("-D").arg(path)))
            .collect();
        assert!(ids[0].contains("@rpath/libmiddle.dylib"));
        assert!(ids[1].contains("@rpath/libleaf.dylib"));

        let out = temp.0.join("out");
        let staged = prepare_macos_runtime(&files, &out);
        assert_eq!(
            staged,
            files
                .iter()
                .map(|path| out.join("runtime").join(path.file_name().unwrap()))
                .collect::<Vec<_>>()
        );
        for (path, bytes) in files.iter().zip(&originals) {
            assert_eq!(
                fs::read(path).unwrap(),
                *bytes,
                "original changed: {}",
                path.display()
            );
        }
        for (path, id) in [&files[0], &files[3]].iter().zip(&ids) {
            assert_eq!(run(Command::new("otool").arg("-D").arg(path)), *id);
        }
        assert_eq!(fs::read(&staged[1]).unwrap(), originals[1]);
        for index in [0, 2, 3] {
            run(Command::new("codesign")
                .args(["--verify", "--strict"])
                .arg(&staged[index]));
        }
        for index in [0, 3] {
            let id = run(Command::new("otool").arg("-D").arg(&staged[index]));
            assert_eq!(
                id.lines().nth(1).unwrap(),
                format!(
                    "@loader_path/{}",
                    staged[index].file_name().unwrap().to_str().unwrap()
                )
            );
        }
        assert!(run(Command::new("otool").arg("-L").arg(&staged[0]))
            .contains("@loader_path/libleaf.dylib"));
        assert!(run(Command::new("otool").arg("-L").arg(&staged[2]))
            .contains("@loader_path/libmiddle.dylib"));
        fs::rename(&source, temp.0.join("hidden originals")).unwrap();
        // No embedded rpath or inherited DYLD_* variables can supply the dylibs.
        run(Command::new(&staged[2])
            .env_clear()
            .current_dir(out.join("runtime")));
    }
}
