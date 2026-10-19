# Rust with Tauri

How Clarity turns a project into a Rust desktop program with Tauri and the web renderer, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```
stack rust web desktop
```

The backend is Rust, the UI is Clarity's web renderer, and the shell is Tauri 2. See [The project file](project.file.md).

## What is generated

Clarity writes a Cargo project into the output folder (`out` by default):

- `Cargo.toml`, `build.rs` and `tauri.conf.json`: the package, Tauri's build step and the window settings.
- `src/modules/<module>.rs`: one Rust module per Clarity module, with its records, enums, functions, actions, windows and handlers. `src/modules.rs` declares them.
- `src/schema.rs`: the migration steps of every database Clarity owns.
- `src/main.rs`: opens and migrates the databases and starts the Tauri application.
- `src/model.json`: the windows as the renderer reads them.
- `ui/`: the web renderer (`index.html`, `runtime.js`, `runtime.css`), a CSS file per theme, and the project's pictures in `ui/images`.
- `icons/icon.png` and a `README.md` that says how to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again. See [Run and Build](project.run-build.md).

## The runtime crate

The generated project carries no runtime sources. It depends on the crate `clarity-runtime` from crates.io, with only the features of the databases it uses: `sqlite`, `postgres`, `mysql` or `mssql`. The first build downloads it.

To build on the runtime in Clarity's repository instead (while you work on the runtime itself), write `.cargo/config.toml` in the output folder:

```
[patch.crates-io]
clarity-runtime = { path = "<Clarity>/runtimes/rust" }
```

Clarity keeps the `.cargo` folder when it writes the project again. Remove it to build on the published crate.

A project with an [engine](code.engines.md) also turns on the crate's `engines` feature, which runs engines with wasmi.

## What to install

- Rust with cargo, from https://rustup.rs
- On Linux, the WebKitGTK development files and pkg-config (Debian and Ubuntu: `libwebkit2gtk-4.1-dev`; see https://tauri.app/start/prerequisites/)

Clarity checks for these itself: run `clarity tools`, or look at Build tools on the Project tab.

## Build and run

**Run** on the Preview tab builds a debug program and starts it; **Build** makes a release program. Without Clarity, in the output folder:

```
cargo build --release
cargo run
```

The release program is `target/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case. A debug build goes to `target/debug/<name>`.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. A click target marked `in new window` opens a second Tauri window. See [Windows](design.windows.md).

## The same on every stack

The web renderer is shared by the Rust, Go, C and C++ stacks, so they cannot differ. The parts of a window behave alike on the web renderer and on Avalonia ([C# with Avalonia](stacks.csharp.md)): the shared cases in `conformance/` say what each part must do, and both renderers run them.
