# Rust with Clarity.UI

How Clarity turns a project into a Rust program drawn by Clarity.UI, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack rust desktop
```

Rust names no UI: it draws its windows with Clarity.UI, as C# does. Clarity.UI is Clarity's own UI framework: it draws the windows itself with Skia in the system's own window, and every stack gets its own version of it (see [C# with Clarity.UI](stacks.csharp.md)). A desktop program is nothing web: there is no webview in it. The webview desktop Rust once had (Tauri, `stack rust webview desktop`) is gone; a project file that still says so is refused with the line to write instead.

The web target serves the same windows to a browser:

```clarity
stack rust web
```

The program is a web server. Its page runs Clarity.UI in the browser (the same library, compiled once for the browser), which draws the windows and asks the program on the server for each handler, edit and engine frame. With `stack rust desktop web` it is one program: it opens its window, and started with `--serve` it serves instead. See [The project file](project.file.md).

## What is generated

With `stack rust desktop` Clarity writes a Cargo project into the output folder (`out` by default):

- `Cargo.toml`: the package, on the crates `clarity-runtime` (the window model, records, queries and migrations) and `clarity-ui` (the drawing, with Skia, in the system's own window).
- `src/modules/<module>.rs`: one Rust module per Clarity module, with its records, enums, functions, actions, windows and handlers. `src/modules.rs` declares them.
- `src/schema.rs`: the migration steps of every database Clarity owns.
- `windows/<Window>.cxaml` and `apps/<App>.cxaml`: each window and each app frame (menu bar, toolbar, ribbon, sidebars, footer) as markup (`.cxaml`, Clarity's own dialect): its parts as elements with their options, as in Clarity (`<Button name="clearKey" at="1,3" span="1" caption="@clearKeyCaption" key="Escape" onclick="clear" />`).
- `themes/<theme>.light.json` and `.dark.json`: each theme the project offers, as Clarity resolves it. Without any, Clarity.UI shows Clarity's own.
- `resources/`: the project's pictures and other resources, carried in the program.
- `src/model.json` and `src/facts.rs`: the data dictionary and the translations, and the facts of the fields Clarity.UI's parts read.
- `src/main.rs`: the program, which answers what the windows ask of it (the runtime's `Program`); it opens and migrates the databases and starts `clarity_ui::desktop` with the files above, which it carries in itself.
- a `README.md` that says how to build the project without Clarity.

With the `web` target the program also has `browser/`, the files its server serves, all carried in the program:

- `index.html`: the page, a canvas Clarity.UI draws on.
- `clarity/files.json`: what Clarity.UI draws from in the browser: the windows and app frames as markup, the themes, the data dictionary, the title and the main window's size; `clarity/resources/` holds the resources.
- `clarity-ui.js`, `clarity-ui.wasm`, `clarity-ui-worker.js` and `clarity-ui-page.js`: Clarity.UI for the browser. Clarity copies them in when it writes the program, from the folder the environment variable `CLARITY_UI_BROWSER` names, else the ones Clarity carries, else the release's archive, fetched once.

`src/main.rs` answers what the browser asks over `/clarity/invoke` with the same program the desktop runs. The program listens at `--address host:port`, else the environment's `CLARITY_ADDRESS`, else the project's port. Requests run side by side, so a long call shows its bar and Stop in the browser too.

Cargo downloads Skia's prebuilt library on the first build; nothing is built from Skia's source. Do not edit these files. Change the project in Clarity; it writes the folder again. See [Run and Build](project.run-build.md).

### Custom parts

A custom part written in Rust (`crate "parts/Gauge"`) is a control of Clarity.UI: it draws on the window's own surface, in its place, and takes the pointer, the wheel, keys and the focus as any part does. Its crate depends on `clarity-ui`; Clarity writes the crate's `src/clarity.rs` from the declaration, with the trait `GaugePart` that the crate's `Gauge` implements:

- `draw(&self, painter, area)`: draws the part with Skia, in the theme's look (`painter.look`).
- `show_value(value, cx)`, `show_<property>(value, cx)` and `on_<hear>(values, cx)`: what the window gives it, typed as the declaration says.
- `click`, `key`, `text`, `wheel`, `pointer_pressed` and the other input methods: each does nothing unless the crate writes it.

From any of them the part speaks to its window through `cx`: `cx.changed(value)` changes what it binds (one edit, which Ctrl+Z takes back, and `on change` runs), and `cx.reached(level)` and the other tells run the handler wired to them. The program registers each part by its name in the generated `src/parts.rs`. Declaring a part writes a starter crate that draws its value in a frame; Clarity never writes over it, only over `src/clarity.rs`. See [Custom parts](design.custom-parts.md).

A program with custom parts builds its dependencies optimized in a debug build too: its `Cargo.toml` says

```toml
[profile.dev.package."*"]
opt-level = 3
```

so a part that runs a game engine or other heavy work (Bevy, say) runs at speed when you try the program with **Run**, while the program's own code stays quick to build. The first debug build takes longer for it; later ones only build what changed.

## The runtime crate

The generated project carries no runtime sources. It depends on the crate `clarity-runtime` from crates.io, with only the features of the databases it uses: `sqlite`, `postgres`, `mysql` or `mssql`. The first build downloads it.

To build on the crates in Clarity's repository instead (while you work on them), set the environment variable `CLARITY_RUNTIME` to the Rust runtime's folder, `<Clarity>/runtimes/rust`, while Clarity generates the project (as `CLARITY_RUNTIME` names `<Clarity>/runtimes/csharp` for C#). Clarity then writes `.cargo/config.toml` in the program's folder, which patches the crates the project uses onto the repository:

```toml
[patch.crates-io]
clarity-runtime = { path = "<Clarity>/runtimes/rust" }
clarity-ui = { path = "<Clarity>/runtimes/rust-ui" }
```

The second line is there only for the desktop program: Clarity.UI's crate is taken from beside the runtime's folder. The file can also be written by hand. Clarity keeps the `.cargo` folder when it writes the project again without the variable; remove it to build on the published crates.

A project with an [engine](code.engines.md) also turns on the crate's `engines` feature, which runs engines with wasmi.

## What to install

- Rust with cargo, from https://rustup.rs

Nothing more: cargo downloads Skia's prebuilt library, and there is no webview to install. Clarity checks for these itself: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder:

```shell
cargo build --release
cargo run
```

The release program is `target/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case. A debug build goes to `target/debug/<name>`.

Each SQLite database opens in the folder the project names for it. Set the environment variable `CLARITY_DATA` to another folder to run the program on other data, such as a copy, without changing the program: each database then opens there under its own file name.

The desktop program can also run without a window, driven step by step: with the environment variable `CLARITY_HEADLESS` set, it draws its windows on Skia's raster surface in its own process, reads one step per line on its standard input as JSON (a click on a part, keys, typed text, a drag, `read`, `shot`) and answers each on its standard output. Clarity's tests drive the every-part project this way (`crates/clarity-gen/tests/everything_live.rs`); `runtimes/rust-ui/src/headless.rs` lists the steps.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens in a desktop window of its own over its window. A click target marked `in new window` opens a second desktop window; only that route does, any other route or `open` to the same window opens it in place. See [Windows](design.windows.md).

## The same on every stack

Clarity.UI in Rust follows Clarity.UI in C# ([C# with Clarity.UI](stacks.csharp.md)) part by part, and every language's web program serves the same Clarity.UI in the browser. The shared cases in `conformance/` say what each part must do, and every renderer runs them.
