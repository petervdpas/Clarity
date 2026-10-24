# C++ with a webview

How Clarity turns a project into a C++20 desktop program that shows the web renderer in a webview, what you need to install, and how to build it with CMake.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack cpp web desktop
```

The backend is C++20, the UI is Clarity's web renderer, and the window is a webview. The project has the same shape as a [C project](stacks.c.md). See [The project file](project.file.md).

## SQLite only

The C++ stack stores records in SQLite only. A data source on PostgreSQL, MySQL or SQL Server stops generation with a message; the server databases come later. See [Data sources](data.sources.md).

## What is generated

Clarity writes a CMake project into the output folder (`out` by default). All generated code is in a namespace named after the program.

- `CMakeLists.txt` and `CMakePresets.json`: the build, with a debug and a release preset.
- `src/records.hpp` and `src/records.cpp`: the enums and records of every module.
- `src/modules/<module>.cpp`: one file per Clarity module, with its functions, actions, data events, windows and handlers, all declared in `src/app.hpp`.
- `src/schema.hpp` and `src/schema.cpp`: the migration steps of every database Clarity owns.
- `src/page.cpp`: the web renderer and the windows, built into the program.
- `src/main.cpp`: opens and migrates the databases and shows the first window.
- `clarity-runtime/`: the C++ runtime and its shell, copied in.
- `README.md`: what to install to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime

As for C, Clarity copies its runtime into every project as `clarity-runtime/`. It holds the runtime (records, storage, queries and the binding to the screen), the shell that opens the window, and the webview library. JSON comes from nlohmann_json, which you install on the system.

A project with an [engine](code.engines.md) also gets the engine host and wasm3, which runs engines, in `clarity-runtime/`.

## What to install

- A C++ compiler: GCC or Clang (on Windows, Visual Studio's Build Tools)
- CMake 3.25 or newer, from https://cmake.org/download
- make or Ninja, to build what CMake writes
- The SQLite development files (Debian and Ubuntu: `libsqlite3-dev`)
- nlohmann-json (Debian and Ubuntu: `nlohmann-json3-dev`; elsewhere https://github.com/nlohmann/json)
- On Linux, the WebKitGTK and GTK 3 development files and pkg-config (Debian and Ubuntu: `libwebkit2gtk-4.1-dev libgtk-3-dev`)

CMake finds SQLite and nlohmann_json itself and the WebKitGTK libraries through pkg-config. Clarity checks for all of these: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Run** on the Preview tab builds a debug program and starts it; **Build** makes a release program. Without Clarity, in the output folder, one command configures and builds:

```shell
cmake --workflow --preset debug
cmake --workflow --preset release
```

The program is `build/debug/<name>` or `build/release/<name>` (with `.exe` on Windows), where `<name>` is the project name in snake case.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. A click target marked `in new window` opens a second webview window. See [Windows](design.windows.md).

## The same on every stack

The web renderer is shared by the Rust, Go, C and C++ stacks, so they cannot differ. The parts of a window behave alike on the web renderer and on Avalonia ([C# with Avalonia](stacks.csharp.md)): the shared cases in `conformance/` say what each part must do, and both renderers run them.
