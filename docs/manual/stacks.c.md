# C with Clarity.UI

How Clarity turns a project into a C17 desktop program whose windows Clarity.UI draws, what you need to install, and how to build it with CMake.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack c desktop
```

The backend is C17 and the windows are drawn by Clarity.UI, Clarity's own UI library, through its C interface (`clarity_ui.h`). `stack c web` serves the application to a browser instead, with Clarity's web screen. See [The project file](project.file.md).

## SQLite only

The C stack stores records in SQLite only. A data source on PostgreSQL, MySQL or SQL Server stops generation with a message; the server databases come later. See [Data sources](data.sources.md).

## What is generated

Clarity writes a CMake project into the output folder (`out` by default):

- `CMakeLists.txt` and `CMakePresets.json`: the build, with a debug and a release preset.
- `src/records.h` and `src/records.c`: the enums and records of every module.
- `src/modules/<module>.c`: one file per Clarity module, with its functions, actions, data events, windows and handlers, all declared in `src/app.h`.
- `src/schema.c`: the migration steps of every database Clarity owns.
- `src/model.c`: the model (the data dictionary, the translations, what each window is besides its look), and `src/facts.c`: what the parts read beside the values (labels, formats, required fields), both built into the program.
- `windows/` and `apps/`: each window, and each app's frame (menu bar, toolbar, ribbon, sidebars, footer), as markup (`.cxaml`, Clarity's own dialect): its parts as elements with their options, as in Clarity (`<Button name="clearKey" at="1,3" span="1" caption="@clearKeyCaption" key="Escape" onclick="clear" />`). `themes/` holds the project's themes as Clarity resolves them, `resources/` its resources. CMake puts these files into the program when it builds it (`clarity_carry` in `CMakeLists.txt`, in the order the project gives them), so a changed window file takes effect on the next build.
- `src/parts.c`: for a project with [custom parts](design.custom-parts.md), each part registered with Clarity.UI.
- `src/main.c`: opens and migrates the databases and shows the first window.
- `clarity-runtime/`: the C runtime, copied in.

A web program (`stack c web`) has `ui/index.html`, Clarity's web screen as one page, and the windows in `ui/windows/` and `ui/apps/` instead.
- `README.md`: what to install to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime

C has no package registry, so Clarity copies its runtime into every project as `clarity-runtime/`. It holds the runtime (records, storage, queries and the binding to the screen), the service that answers Clarity.UI and the web screen (`service/`), the thin layer on Clarity.UI (`desktop/`), and cJSON for JSON.

## Clarity.UI's library

The desktop program draws with Clarity.UI's library, `clarity_ui`. The build takes it from the folder the environment variable `CLARITY_UI` names (or `cmake -DCLARITY_UI=<folder>`): `include/clarity_ui.h`, and in `lib/` the library for the system (`libclarity_ui.so` on Linux, `libclarity_ui.dylib` on macOS, `clarity_ui.dll` with its import library on Windows). Each build copies the library next to the program, where the program finds it; ship the two together. With `CLARITY_HEADLESS` set the program runs without a window, driven by steps on its standard input.

A project with an [engine](code.engines.md) also gets the engine host and wasm3, which runs engines, in `clarity-runtime/`.

## What to install

- A C compiler: GCC or Clang (on Windows, Visual Studio's Build Tools)
- CMake 3.25 or newer, from https://cmake.org/download
- make or Ninja, to build what CMake writes
- The SQLite development files (Debian and Ubuntu: `libsqlite3-dev`)
- For a desktop program, Clarity.UI's library, in the folder `CLARITY_UI` names (above)
- For a project whose code calls web APIs, the libcurl development files (Debian and Ubuntu: `libcurl4-openssl-dev`)

CMake finds SQLite itself. Clarity checks for all of these: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder, one command configures and builds:

```shell
cmake --workflow --preset debug
cmake --workflow --preset release
```

The program is `build/debug/<name>` or `build/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. A click target marked `in new window` opens a desktop window of its own. See [Windows](design.windows.md).

## The same on every stack

Clarity.UI is one library for every language: a C program's windows draw and behave as the same windows of [Rust](stacks.rust.md) and [C#](stacks.csharp.md) do. The shared cases in `conformance/` say what each part must do, and every renderer runs them.
