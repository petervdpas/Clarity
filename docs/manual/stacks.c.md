# C with a webview

How Clarity turns a project into a C17 desktop program that shows the web renderer in a webview, what you need to install, and how to build it with CMake.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack c web desktop
```

The backend is C17, the UI is Clarity's web renderer, and the window is a webview. See [The project file](project.file.md).

## SQLite only

The C stack stores records in SQLite only. A data source on PostgreSQL, MySQL or SQL Server stops generation with a message; the server databases come later. See [Data sources](data.sources.md).

## What is generated

Clarity writes a CMake project into the output folder (`out` by default):

- `CMakeLists.txt` and `CMakePresets.json`: the build, with a debug and a release preset.
- `src/records.h` and `src/records.c`: the enums and records of every module.
- `src/modules/<module>.c`: one file per Clarity module, with its functions, actions, data events, windows and handlers, all declared in `src/app.h`.
- `src/schema.c`: the migration steps of every database Clarity owns.
- `src/page.c`: the web renderer and the windows, built into the program.
- `src/main.c`: opens and migrates the databases and shows the first window.
- `clarity-runtime/`: the C runtime and its shell, copied in.
- `README.md`: what to install to build the project without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime

C has no package registry, so Clarity copies its runtime into every project as `clarity-runtime/`. It holds the runtime (records, storage, queries and the binding to the screen), the shell that opens the window, cJSON for JSON, and the webview library. The webview's own code is one C++ file, so you need a C++ compiler for it too.

A project with an [engine](code.engines.md) also gets the engine host and wasm3, which runs engines, in `clarity-runtime/`.

## What to install

- A C compiler: GCC or Clang (on Windows, Visual Studio's Build Tools)
- A C++ compiler for the webview's one C++ file: GCC (g++) or Clang (clang++)
- CMake 3.25 or newer, from https://cmake.org/download
- make or Ninja, to build what CMake writes
- The SQLite development files (Debian and Ubuntu: `libsqlite3-dev`)
- On Linux, the WebKitGTK and GTK 3 development files and pkg-config (Debian and Ubuntu: `libwebkit2gtk-4.1-dev libgtk-3-dev`)

CMake finds SQLite itself and the WebKitGTK libraries through pkg-config. Clarity checks for all of these: run `clarity tools`, or look at Build tools on the Project tab.

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
