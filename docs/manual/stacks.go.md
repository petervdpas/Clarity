# Go with Clarity.UI

How Clarity turns a project into a Go desktop program drawn by Clarity.UI, or a Go web server, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack go desktop
```

The backend is Go and the windows are drawn by Clarity.UI, Clarity's own UI, in the program's own process. `stack go web` makes a web server that shows the windows in a browser with Clarity's web renderer, and `stack go desktop cli` adds the command-line program. The desktop and the web program are not generated together yet. See [The project file](project.file.md).

## What is generated

Clarity writes a Go module into the output folder (`out` by default), all in package `main`:

- `go.mod` and `go.sum`: the module, its requirements and their sums, as `go mod tidy` would write them.
- `<module>_module.go`: one file per Clarity module, with its enums, records, functions, actions, data events, windows and handlers.
- `schema.go`: the migration steps of every database Clarity owns.
- `main.go`: opens and migrates the databases and runs the windows on Clarity.UI (or serves them, for the web target).
- `ui/windows/` and `ui/apps/`: each window, and each app's frame (menu bar, toolbar, ribbon, sidebars, footer), as markup (`.cxaml`, Clarity's own dialect): its parts as elements with their options, as in Clarity (`<Button name="clearKey" at="1,3" span="1" caption="@clearKeyCaption" key="Escape" onclick="clear" />`). Change a window in Clarity; Generate writes these files again.
- `ui/themes/` and `ui/resources/`: the project's themes as Clarity resolves them, and its pictures and other resources. The web target has Clarity's screen in `ui/` instead (`index.html`, `runtime.js`, `runtime.css`).
- `model.json`: the data dictionary and the translations; `facts.json`: what the parts read beside the values (each field's label, format and range).
- `engines/`: the [engines](code.engines.md), those beside the screen too, which the program runs.
- `parts.go` and `parts/`: the [custom parts](design.custom-parts.md) written in Go, each a module of its own.
- `README.md`: how to build the module without Clarity.

The program embeds all of it. Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime module

The generated module carries no runtime sources. It requires the Go module `github.com/petervdpas/Clarity/runtimes/go`, which has these packages:

- `clarity`: records, storage (SQLite included), queries, migrations and the binding to the screen.
- `shell`: the commands of the screen, and the web server.
- `ui`: the desktop on Clarity.UI, calling its library through cgo.
- `postgresql`, `mysql` and `sqlserver`: one per server database. A generated module imports only the ones its data sources use, and its go.mod leaves out the others' dependencies.
- `engine`: runs [engines](code.engines.md) with wazero, imported only when the project has one.

The first build downloads them. To build on the runtime in Clarity's repository instead (while you work on the runtime itself), make a workspace in the output folder:

```shell
go work init .
go work edit -replace github.com/petervdpas/Clarity/runtimes/go=<Clarity>/runtimes/go
```

Clarity keeps `go.work` when it writes the module again. Remove it to build on the published module.

## Clarity.UI's library

Clarity.UI is one library with a C interface, the same for every language. The desktop program finds it in the folder the environment variable `CLARITY_UI` names: `include/clarity_ui.h`, and in `lib/` the library for the system (`libclarity_ui.so` on Linux, `libclarity_ui.dylib` on macOS, `clarity_ui.dll` on Windows). Clarity's build hands that folder to cgo. When `CLARITY_UI` is not set, Clarity uses the library its installer carries; a Clarity without it fetches the library itself on the first build: the archive of its library version for the system from the release `runtimes/rust-ui-ffi/v<version>` on GitHub (https://github.com/petervdpas/Clarity/releases), checked against the release's `SHA256SUMS` and unpacked into your cache folder (`~/.cache/clarity/clarity-ui/<version>/<system>` on Linux, `~/Library/Caches/...` on macOS, `%LOCALAPPDATA%\clarity\clarity-ui\...` on Windows), once per version. A `CLARITY_UI` you set always wins.

cgo downloads nothing, so to build without Clarity, take `clarity-ui-<version>-<system>.tar.gz` from that release (`<system>` is one of `linux-x64`, `linux-arm64`, `win-x64`, `win-arm64`, `osx-x64`, `osx-arm64`; the generated README names the version), check it against `SHA256SUMS`, unpack it and set `CLARITY_UI` to the folder it holds. Then, on Linux:

```shell
export CGO_CFLAGS="-I$CLARITY_UI/include"
export CGO_LDFLAGS="-L$CLARITY_UI/lib -Wl,-rpath,\$ORIGIN -Wl,-rpath,$CLARITY_UI/lib"
```

On macOS `@executable_path` takes the place of `$ORIGIN`; on Windows `-L` alone. The program finds the library beside it, else in that folder: ship the library beside the program (on Windows it must be there, or on `PATH`). In Clarity's repository it is built with `cargo build --release -p clarity-ui-ffi`.

## What to install

- Go, from https://go.dev/dl
- a C compiler for cgo, as the desktop program calls Clarity.UI's library: GCC or Clang (on Windows, MSYS2's UCRT64 shell)
- Clarity.UI's library: Clarity carries it (or fetches it on the first build), or `CLARITY_UI` names it (above)

Clarity checks for these itself: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder:

```shell
go build -trimpath -ldflags="-s -w" -o bin/<name> .
go run .
```

The program is `bin/<name>` (with `.exe` on Windows), where `<name>` is the module name. With the environment variable `CLARITY_HEADLESS` set, the desktop program shows no window and takes its steps from standard input, one JSON line each (`{"do":"click","part":"save"}`), for tests and scripts.

## Windows on screen

`open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens in a desktop window of its own, and a click target marked `in new window` opens its window in a desktop window of its own. See [Windows](design.windows.md).

## The same on every stack

Clarity.UI is one library for every language, so the Go desktop draws exactly what the Rust, C, C++ and C# desktops draw; the shared cases in `conformance/` say what each part must do.
