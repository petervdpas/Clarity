# Rust with Tauri

How Clarity turns a project into a Rust desktop program with Tauri and the web renderer, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack rust web desktop
```

The backend is Rust, the UI is Clarity's web renderer, and the shell is Tauri 2. See [The project file](project.file.md).

## What is generated

Clarity writes a Cargo project into the output folder (`out` by default):

- `Cargo.toml`, `build.rs` and `tauri.conf.json`: the package, Tauri's build step and the window settings.
- `src/modules/<module>.rs`: one Rust module per Clarity module, with its records, enums, functions, actions, windows and handlers. `src/modules.rs` declares them.
- `src/schema.rs`: the migration steps of every database Clarity owns.
- `src/main.rs`: opens and migrates the databases and starts the Tauri application.
- `src/model.json`: the frame, the data dictionary and the translations, for the screen.
- `ui/`: the screen, a Svelte project of its own: each window as a file in `ui/src/windows/`, its parts as elements with their options as in Clarity (`<Button name="saveKey" at="3,5" span={2} caption="@saveCaption" onclick="save" />`); Clarity's screen package in `ui/clarity-ui/`, which the project carries; a CSS file per theme and the project's pictures in `ui/public/`. npm and Vite build it into `ui/dist/`, which the program embeds.
- `icons/icon.png` and a `README.md` that says how to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again. See [Run and Build](project.run-build.md).

## The runtime crate

The generated project carries no runtime sources. It depends on the crate `clarity-runtime` from crates.io, with only the features of the databases it uses: `sqlite`, `postgres`, `mysql` or `mssql`. The first build downloads it.

To build on the runtime in Clarity's repository instead (while you work on the runtime itself), write `.cargo/config.toml` in the output folder:

```toml
[patch.crates-io]
clarity-runtime = { path = "<Clarity>/runtimes/rust" }
```

Clarity keeps the `.cargo` folder when it writes the project again. Remove it to build on the published crate.

A project with an [engine](code.engines.md) also turns on the crate's `engines` feature, which runs engines with wasmi.

## What to install

- Rust with cargo, from https://rustup.rs
- Node.js with npm, from https://nodejs.org, which builds the screen
- On Linux, the WebKitGTK development files and pkg-config (Debian and Ubuntu: `libwebkit2gtk-4.1-dev`; see https://tauri.app/start/prerequisites/)

Clarity checks for these itself: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Both build the screen first. Without Clarity, in the output folder:

```shell
npm --prefix ui install
npm --prefix ui run build
cargo build --release
cargo run
```

The release program is `target/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case. A debug build goes to `target/debug/<name>`.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. A click target marked `in new window` opens a second Tauri window. See [Windows](design.windows.md).

## The same on every stack

The screen package is shared by the Rust, Go, C and C++ stacks, so they cannot differ; a window drawn from its markup draws exactly what the same window drawn from the model does, which a test checks for every window of every example. The parts of a window behave alike on the web renderer and on Avalonia ([C# with Avalonia](stacks.csharp.md)): the shared cases in `conformance/` say what each part must do, and both renderers run them.
