# Rust with Clarity.UI or Tauri

How Clarity turns a project into a Rust program, drawn by Clarity.UI or by the shared web screen in Tauri, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack rust desktop
```

Without a UI word, Rust draws its windows with Clarity.UI, as C# does. Clarity.UI is Clarity's own UI framework: it draws the windows itself with Skia, and every stack gets its own version of it (see [C# with Clarity.UI](stacks.csharp.md)). The Rust version is in progress. It opens the main window with its title and size and draws the first parts; a part it does not draw yet shows its kind in its place until it arrives. The web target of Rust on Clarity.UI comes later: `stack rust web` is refused for now.

Name `webview` to keep the shared web screen, in a Tauri window or in a browser:

```clarity
stack rust webview desktop
stack rust webview web
```

The rest of this page is about the shared web screen, unless it says Clarity.UI. See [The project file](project.file.md).

## What is generated with Clarity.UI

With `stack rust desktop` Clarity writes a Cargo project without Tauri:

- `Cargo.toml`: the package, on the crates `clarity-runtime` (the window model, records, queries and migrations) and `clarity-ui` (the drawing, with Skia, in the system's own window).
- `src/modules/<module>.rs`: one Rust module per Clarity module, as for the web screen.
- `windows/<Window>.cxaml` and `apps/<App>.cxaml`: each window and each app frame as markup, the same markup the web screen and C# read.
- `themes/<theme>.light.json` and `.dark.json`: each theme the project offers, as Clarity resolves it. Without any, Clarity.UI shows Clarity's own.
- `src/main.rs`: the program, which answers what the windows ask of it (the runtime's `Program`); it opens and migrates the databases and starts `clarity_ui::desktop` with the files above and the model with its translations, which it carries in itself.

Cargo downloads Skia's prebuilt library on the first build; nothing is built from Skia's source.

### Custom parts on Clarity.UI

A custom part written in Rust (`crate "parts/Gauge"`) is a control of Clarity.UI: it draws on the window's own surface, in its place, and takes the pointer, the wheel, keys and the focus as any part does. Its crate depends on `clarity-ui`; Clarity writes the crate's `src/clarity.rs` from the declaration, with the trait `GaugePart` that the crate's `Gauge` implements:

- `draw(&self, painter, area)`: draws the part with Skia, in the theme's look (`painter.look`).
- `show_value(value, cx)`, `show_<property>(value, cx)` and `on_<hear>(values, cx)`: what the window gives it, typed as the declaration says.
- `click`, `key`, `text`, `wheel`, `pointer_pressed` and the other input methods: each does nothing unless the crate writes it.

From any of them the part speaks to its window through `cx`: `cx.changed(value)` changes what it binds (one edit, which Ctrl+Z takes back, and `on change` runs), and `cx.reached(level)` and the other tells run the handler wired to them. The program registers each part by its name in the generated `src/parts.rs`. Declaring a part writes a starter crate that draws its value in a frame; Clarity never writes over it, only over `src/clarity.rs`. See [Custom parts](design.custom-parts.md).

## What is generated with the web screen

With `stack rust webview desktop` Clarity writes a Cargo project into the output folder (`out` by default):

- `Cargo.toml`, `build.rs` and `tauri.conf.json`: the package, Tauri's build step and the window settings.
- `src/modules/<module>.rs`: one Rust module per Clarity module, with its records, enums, functions, actions, windows and handlers. `src/modules.rs` declares them.
- `src/schema.rs`: the migration steps of every database Clarity owns.
- `src/main.rs`: opens and migrates the databases and starts the Tauri application.
- `src/model.json`: the data dictionary, the translations, and what each window is besides its look, for the screen.
- `ui/`: Clarity's screen (`index.html`, `runtime.js`, `runtime.css`), a CSS file per theme, and the project's pictures in `ui/resources`. Nothing in it is built.
- `ui/windows/` and `ui/apps/`: each window, and each app's frame (menu bar, toolbar, ribbon, sidebars, footer), as markup (`.cxaml`, Clarity's own dialect), read by the screen when the application starts: its parts as elements with their options, as in Clarity (`<Button name="clearKey" at="1,3" span="1" caption="@clearKeyCaption" key="Escape" onclick="clear" />`). Change a window in Clarity; Generate writes these files again.
- `icons/icon.png` and a `README.md` that says how to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again. See [Run and Build](project.run-build.md).

## The runtime crate

The generated project carries no runtime sources. It depends on the crate `clarity-runtime` from crates.io, with only the features of the databases it uses: `sqlite`, `postgres`, `mysql` or `mssql`. The first build downloads it.

To build on the runtime in Clarity's repository instead (while you work on the runtime itself), write `.cargo/config.toml` in the output folder:

```toml
[patch.crates-io]
clarity-runtime = { path = "<Clarity>/runtimes/rust" }
```

With Clarity.UI, add the drawing crate beside it: `clarity-ui = { path = "<Clarity>/runtimes/rust-ui" }`.

Clarity keeps the `.cargo` folder when it writes the project again. Remove it to build on the published crate.

A project with an [engine](code.engines.md) also turns on the crate's `engines` feature, which runs engines with wasmi.

## What to install

- Rust with cargo, from https://rustup.rs
- For the web screen on Linux, the WebKitGTK development files and pkg-config (Debian and Ubuntu: `libwebkit2gtk-4.1-dev`; see https://tauri.app/start/prerequisites/)

Clarity.UI needs nothing more: cargo downloads Skia's prebuilt library. Clarity checks for these itself: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder:

```shell
cargo build --release
cargo run
```

The release program is `target/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case. A debug build goes to `target/debug/<name>`.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. With the web screen, a click target marked `in new window` opens a second Tauri window. See [Windows](design.windows.md).

## The same on every stack

The screen is shared by the Rust, Go, C and C++ stacks, so they cannot differ; a window drawn from its markup draws exactly what the same window drawn from the model does, which a test checks for every window of every example. The parts of a window behave alike on the web renderer and on C# ([C# with Clarity.UI](stacks.csharp.md)): the shared cases in `conformance/` say what each part must do, and every renderer runs them.
