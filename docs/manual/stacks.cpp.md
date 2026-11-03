# C++ with Clarity.UI

How Clarity turns a project into a C++20 desktop program whose windows Clarity.UI draws, or a C++ web server, what you need to install, and how to build it with CMake.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack cpp desktop
```

The backend is C++20 and the windows are drawn by Clarity.UI, Clarity's own UI, in the program's own process. `stack cpp web` makes a web server that shows the windows in a browser with Clarity's web renderer, and `stack cpp desktop cli` adds the command-line program. The project has the same shape as a [C project](stacks.c.md). See [The project file](project.file.md).

## SQLite only

The C++ stack stores records in SQLite only. A data source on PostgreSQL, MySQL or SQL Server stops generation with a message; the server databases come later. See [Data sources](data.sources.md).

## What is generated

Clarity writes a CMake project into the output folder (`out` by default). All generated code is in a namespace named after the program.

- `CMakeLists.txt` and `CMakePresets.json`: the build, with a debug and a release preset.
- `src/records.hpp` and `src/records.cpp`: the enums and records of every module.
- `src/modules/<module>.cpp`: one file per Clarity module, with its functions, actions, data events, windows and handlers, all declared in `src/app.hpp`.
- `src/schema.hpp` and `src/schema.cpp`: the migration steps of every database Clarity owns.
- `src/page.cpp`: the model (the data dictionary, the translations, what each window is besides its look) and the facts the parts read beside the values (each field's label, format and range), built into the program.
- `ui/windows/` and `ui/apps/`: each window, and each app's frame (menu bar, toolbar, ribbon, sidebars, footer), as markup (`.cxaml`, Clarity's own dialect): its parts as elements with their options, as in Clarity (`<Button name="clearKey" at="1,3" span="1" caption="@clearKeyCaption" key="Escape" onclick="clear" />`). Change a window in Clarity; Generate writes these files again. CMake builds them into the program, so a changed window file takes effect on the next build.
- `ui/themes/` and `ui/resources/`: the project's themes as Clarity resolves them, and its pictures and other resources, built into the program the same way. The web target has Clarity's screen as one page instead, `ui/index.html`.
- `src/parts.cpp` and `parts/`: the [custom parts](design.custom-parts.md), each a library of its own folder.
- `src/main.cpp`: opens and migrates the databases and runs the windows on Clarity.UI (or serves them, for the web target).
- `clarity-runtime/`: the C++ runtime, copied in.
- `README.md`: what to install to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime

As for C, Clarity copies its runtime into every project as `clarity-runtime/`. It holds the runtime (records, storage, queries and the binding to the screen), the service that answers the screen's commands, the web server, and the desktop program with `ui/ui.hpp`, the C++ layer over Clarity.UI's library. JSON comes from nlohmann_json, which you install on the system.

A project with an [engine](code.engines.md) also gets the engine host and wasm3, which runs engines, in `clarity-runtime/`.

## What to install

- A C++ compiler: GCC or Clang (on Windows, Visual Studio's Build Tools)
- CMake 3.25 or newer, from https://cmake.org/download
- make or Ninja, to build what CMake writes
- The SQLite development files (Debian and Ubuntu: `libsqlite3-dev`)
- nlohmann-json (Debian and Ubuntu: `nlohmann-json3-dev`; elsewhere https://github.com/nlohmann/json)
- Clarity.UI's library, for a desktop program: a folder with `include/clarity_ui.h` and, in `lib/`, the library for the system, named by the environment variable `CLARITY_UI` (see below)
- For a project whose code calls web APIs, the libcurl development files (Debian and Ubuntu: `libcurl4-openssl-dev`)

CMake finds SQLite and nlohmann_json itself. Clarity checks for all of these: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Clarity.UI's library

Clarity.UI is one library with a C interface (`libclarity_ui.so` on Linux, `libclarity_ui.dylib` on macOS, `clarity_ui.dll` with its import library on Windows) and its header `clarity_ui.h`. CMake reads the folder from the environment variable `CLARITY_UI`, or from `cmake -DCLARITY_UI=<folder>`: `include/clarity_ui.h` and `lib/` with the library. In Clarity's repository it is built with `cargo build --release -p clarity-ui-ffi` (into `target/release`, the header in `runtimes/rust-ui-ffi/include`). Each build copies the library next to the program, where the program finds it when it runs; copy it along with the program.

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder, one command configures and builds:

```shell
cmake --workflow --preset debug
cmake --workflow --preset release
```

The program is `build/debug/<name>` or `build/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. A click target marked `in new window` opens a second desktop window. See [Windows](design.windows.md).

## The same on every stack

Clarity.UI is one library for every language, so the desktop programs of Rust, C#, Go, C and C++ draw alike. The web target shows the shared web screen until Clarity.UI draws in the browser. The shared cases in `conformance/` say what each part must do, and every runtime runs them.
