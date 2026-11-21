# Preview and hot reload

How `cargo elegance preview` works, its exact commands and flags, what survives a reload, its limits, and how an agent
without a display can still verify a UI. Sources: `elegance-rs/cargo-elegance/src/main.rs`,
`elegance-rs/elegance/src/{preview,hot_reload}.rs`, `elegance/apps/preview/main.cpp`, docs `guides/hot-reload.mdx`,
`guides/debugging.mdx`, `elegance-rs/docs/building.md`.

## Contents

- [Install and run](#install-and-run)
- [Writing previews](#writing-previews)
- [Commands and flags](#commands-and-flags)
- [How it works](#how-it-works)
- [What triggers a reload; what survives](#what-triggers-a-reload-what-survives)
- [Limits and requirements](#limits-and-requirements)
- [Verifying a UI without a display](#verifying-a-ui-without-a-display)
- [Debugging a running window](#debugging-a-running-window)
- [Troubleshooting](#troubleshooting)

## Install and run

```sh
cargo install --git https://github.com/EleganceUI/elegance-rs cargo-elegance   # once
cd my-app
cargo elegance preview            # window with a list of #[preview]s and the selected one
cargo elegance preview --root     # the #[preview(root)] view: the whole app
```

`cargo elegance` is a cargo subcommand (`cargo-elegance` binary). The first preview builds Elegance from source as a
shared library (the `dynamic` feature), so it can take a long time (the build guide says roughly half an hour if vcpkg
must compile Dawn). Later runs are incremental. Requires a display: the host is a GUI window.

## Writing previews

```rust
#[preview("Empty list")]            // named preview, shown in the list
fn empty_list() -> impl IntoView { elg! { TodoList(todos: vec![]) } }

#[preview("Dark mode")]
fn dark() -> impl IntoView { elg! { TodoList(todos: sample()).color_scheme(.dark) } }

#[preview(root)]                    // the whole app, used by --root (one per module)
fn app() -> impl IntoView { elg! { Root() } }
```

- Put them anywhere in the crate (they are sorted by file and line). They take no arguments and return `impl IntoView`.
- Without the `preview` feature (every ordinary build) `#[preview]` functions compile to nothing, so they cost nothing.
  The function itself is kept, so it is not reported as dead code.
- `#[preview(root, app_hash = elegance::layout_hash!(Store, Settings))]`: for apps whose host shares types with the
  module (see below). The attribute forms are exercised in `elegance/tests/hot_reload.rs`.
- `--root` needs a `#[preview(root)]`, otherwise the module panics with "this module has no root view; mark one with
  #[preview(root)]".
- Your preview does not need the app's `main`; a binary crate works as is (the binary is built as a library for the
  preview, `fn main` is allowed to be dead code).
- Assets: use `asset!("images/x.png")` (embedded from `<crate>/assets/`), which works in the app, previews and installs.
  A relative file path resolves against the current directory (previews run with the package folder as cwd).

## Commands and flags

`cargo elegance <command> [options]`. Options accept `--name value` or `--name=value`.

| Command | What it does |
| --- | --- |
| `preview [--root] [options]` | Builds the views module, starts the host `elegance-preview` on it, watches and rebuilds. |
| `build-module [options]` | Builds the views module and prints its path. A compile check of the preview build; no window. This is the command the host reruns on file changes. |
| `help` / `--help` / `-h` | Usage. |

| Option | Meaning |
| --- | --- |
| `--root` | (preview only) show the `#[preview(root)]` view instead of the list of previews |
| `-p`, `--package <name>` | package in a workspace. Default: the package whose folder holds the current directory, or the only one |
| `--bin <name>` | binary target. Default: the only bin, or the bin named like the package; a package with several bins needs `--bin`; a package with only a lib builds the lib |
| `--manifest-path <file>` | which Cargo.toml |
| `--release` | release profile (profile folder `release`, otherwise `debug`) |
| `-F`, `--features <list>` | extra cargo features (may repeat). `elegance/preview` is always added |
| `--watch <folder>` | extra folders to watch besides `<package>/src` and `<package>/assets` (may repeat) |

The package must depend directly on `elegance` (a renamed dependency is detected). Anything else gives
`error: package \`x\` doesn't depend on elegance`.

Examples:

```sh
cargo elegance preview -p my-app --bin my-app --release
cargo elegance preview --features demo-data --watch ../shared-ui/src
cargo elegance build-module --manifest-path apps/app/Cargo.toml   # prints .../target/elegance-preview/<name>.so|.dll|.dylib
```

The host itself (`elegance-preview`, built from `elegance/apps/preview/main.cpp`) takes:
`[--root] [--smoke-test] [--module <file> [--build "<command>"] [--watch <folder>]...]`; any other argument prints the
usage and exits 2. `cargo elegance preview` runs it as `elegance-preview [--root] --module <module> --build "<cargo-elegance elegance build-module ...>" --watch <dir>...`
with the package folder as the working directory. The window is titled "Preview - <module name>", 960 x 680.

## How it works

1. `cargo-elegance` runs `cargo build` for your package with itself as `RUSTC_WORKSPACE_WRAPPER`. The wrapper compiles
   just your crate with `--crate-type cdylib` instead of `bin` (or lib), and enables `elegance/preview`, which exports
   the module entry `elg_module_entry` and registers the `#[preview]` functions. Dependencies, build scripts, features,
   `CARGO_MANIFEST_DIR` and `asset!` behave as in a normal build.
2. It uses its own target dir, `target/elegance-preview/`, so your ordinary `cargo build`/`cargo run` are not disturbed.
   The finished module is copied to `target/elegance-preview/<bin-or-package-name><DLL suffix>` (only when changed); the
   host loads copies (from `elegance-reload/<pid>-<n>` in the system temp folder), never the file in place.
3. The host `elegance-preview` is expected at `target/elegance-preview/<debug|release>/elegance-preview[.exe]`;
   `elegance-sys` places it there beside the shared Elegance library when the `dynamic` feature is built.
4. The host shows a list of previews (or the root view), watches the folders, and on a change reruns the build command
   and loads the new module. A small badge shows build progress and the first error line; on a failed build a panel
   titled "The build failed" shows the full compiler output and the old views and their state stay as they were.
   A build in progress is stopped if you close the host.

## What triggers a reload; what survives

- Trigger: a file change in `<package>/src`, `<package>/assets`, or any `--watch` folder. Edits elsewhere (for example a
  path dependency outside those folders, or `Cargo.toml`) are not watched: add `--watch` or restart.
- Not preserved: view state. State (`State`, `Computed`, effects, timers) made inside your views starts over, because
  the code that made it was replaced. Data that must last has to live in the host and be passed in, or sit in the
  environment; environment keys and layout value keys declared in the module still match the host's after a reload
  (identified by name and value type). With `cargo elegance preview` the host is the generic `elegance-preview`, so
  every reload resets view state. Design previews with sample data in the `#[preview]` function.
- Types shared between a custom host and the module (stores, environment values) must stay identical. Give both sides the
  same hash: `elegance::layout_hash!(TypeA, TypeB)` (const, FNV-1a of sizes and alignments) for
  `#[preview(root, app_hash = ...)]` and `HotReload::app_hash`; a mismatch asks for a restart rather than mounting.
  `0` (default) means no check.
- Each load stays in memory until the host quits; long sessions with many reloads use more memory.

## Limits and requirements

| Item | Detail |
| --- | --- |
| Source build of Elegance | Previews use the `dynamic` feature: Elegance is built from source (CMake, vcpkg, a C++ toolchain), not downloaded. Prebuilt packages are static and cannot be used. Env: `ELEGANCE_ROOT`, `ELEGANCE_SHARED_BUILD_DIR`, `ELEGANCE_AUTO_BUILD=0` (see building.md). |
| Missing host | `error: .../elegance-preview not found. elegance-sys copies it from the shared Elegance build it links: build elegance-preview there (cmake --build build/dev-release --target elegance-preview), or unset ELEGANCE_SHARED_BUILD_DIR`. Build the `elegance-preview` target in that Elegance build, then retry. |
| Same toolchain / Elegance | Module and host must use the same compiler and the same Elegance; a module built against another Elegance is refused. |
| Windows | The host loads `elegance.dll`; module is a `.dll`. |
| Display | Needs a window system and GPU. No headless mode, no screenshot output exists in cargo-elegance or the host (searched the host source, `include/`, `src/`). |
| Custom host in your own app | Needs `elegance` with feature `dynamic`, then `HotReload::new(module).build_command(..).watch([..]).show(HotReloadShow::Previews).app_hash(..)` (also usable as `HotReload(path)` in `elg!`). `cargo elegance preview` is simpler. |
| Docs gap | The docs' Rust section covers only `#[preview]` and `cargo elegance preview`; `build-module`, `-F`, `HotReload`, `app_hash` and `layout_hash!` are only in code comments. |

## Verifying a UI without a display

Nothing in the tooling captures screenshots or runs headless rendering. What is real:

| Technique | What it checks | Notes |
| --- | --- | --- |
| `cargo check` / `cargo build` | views type-check: the `elg!` DSL, prop types, leading-dot names, exhaustive `ForEach` ids | fastest feedback; the DSL turns unknown dot-names into compile errors |
| `cargo elegance build-module` | the app also builds as the preview module (cdylib + `preview` feature) | no window; fails if the crate does not depend on `elegance` or has several bins without `--bin` |
| `cargo test` with type-check-only tests | views are built only to type-check; nothing opens a window | pattern used by `elegance/tests/dsl.rs`: construct views in `#[allow(dead_code)]` functions or tests without running an `App` |
| Unit tests of your own logic | state transforms written as plain functions over data (`Vec<Todo>` etc.) | keep logic out of closures so it can be tested; `State` panics off the UI thread, so test pure functions rather than states |
| `elegance-preview --smoke-test --module <file>` | the host opens its window, builds the module view once and quits at the first body (CI runs `elegance-preview --smoke-test`) | still needs a display/GPU; on Windows it also suppresses crash dialogs so a failed check fails the run on stderr. Run the host binary directly; `cargo elegance preview` does not pass this flag. How deep it checks a module is unverified |
| Logs on stderr | `[elg:info]`, `[elg:warning]`, `[elg:error]` lines; missing images/fonts show here | run the app from a terminal and read stderr |
| `App::on_error` | panics and errors in bodies/effects/handlers are logged and the app goes on by default; with `ErrorAction::Quit`, `run()` returns/resumes the failure, so a scripted run fails loudly | see gotchas.md |

If an agent has a display (or Xvfb with a Vulkan driver; unverified), `cargo elegance preview` plus the inspector (F12)
is the visual loop. Otherwise tell the user what was verified (type-check, module build, tests) and what was not (looks).

## Debugging a running window

- F12 opens the inspector (tree with sizes, selected view's offered size/fonts/colors, pick mode, frame stats). Needs a window.
- Env vars documented in `guides/debugging.mdx`: `ELG_SHOW_DAMAGE=1` (tint redrawn regions magenta), `ELG_FULL_REDRAW=1`
  (always redraw everything). Inconsistency: I found neither name in the C++ sources or design.md (grepped
  `/workspaces/mikros/elegance`), so treat them as documented but unverified in code.
- Other env vars that exist in the C++ code: `ELG_GPU_BACKEND` (pick a GPU backend), `ELG_PRESENT` (Windows:
  `swapchain` or `surface`), `ELG_IMAGE_CACHE=off`, `ELG_GLYPH_CACHE`, `ELG_GPU_LABELS=1`.

## Troubleshooting

| Symptom | Likely cause / fix |
| --- | --- |
| `has several binaries; pick one with --bin` | add `--bin <name>` |
| `pick a package with -p` | workspace root: pass `-p <package>` |
| `doesn't depend on elegance` | the package has no normal `elegance` dependency |
| `the build didn't say where it put ...` | the wrapper did not see the crate being built as a module; check `--bin`/`-p` match the crate that holds the previews |
| Preview list is empty | no `#[preview]` in the built crate, or the feature set differs: pass the same `--features` |
| Change not picked up | the file is outside `src/`, `assets/` and `--watch` folders |
| View state resets on each save | expected, see above |
| Linux app cannot find `libaccesskit.so` (normal `cargo run`, not preview) | add `.cargo/config.toml`: `[target.'cfg(target_os = "linux")'] rustflags = ["-C", "link-args=-Wl,-rpath,$ORIGIN"]` |
