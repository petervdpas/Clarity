# Go with Wails

How Clarity turns a project into a Go desktop program with Wails v3 and the web renderer, what you need to install, and how to build it.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack go webview desktop
```

The backend is Go, the UI is Clarity's web renderer, and the shell is Wails v3. See [The project file](project.file.md).

## What is generated

Clarity writes a Go module into the output folder (`out` by default), all in package `main`:

- `go.mod` and `go.sum`: the module, its requirements and their sums, as `go mod tidy` would write them.
- `<module>_module.go`: one file per Clarity module, with its enums, records, functions, actions, data events, windows and handlers.
- `schema.go`: the migration steps of every database Clarity owns.
- `main.go`: opens and migrates the databases and starts the Wails application.
- `ui/`: Clarity's screen (`index.html`, `runtime.js`, `runtime.css`), a CSS file per theme and the project's pictures. The program embeds it; nothing in it is built.
- `ui/windows/` and `ui/apps/`: each window, and each app's frame (menu bar, toolbar, ribbon, sidebars, footer), as markup (`.cxaml`, Clarity's own dialect), read by the screen when the application starts: its parts as elements with their options, as in Clarity (`<Button name="clearKey" at="1,3" span="1" caption="@clearKeyCaption" key="Escape" onclick="clear" />`). Change a window in Clarity; Generate writes these files again.
- `model.json`: the data dictionary, the translations, and what each window is besides its look (its data, events and title), for the screen.
- `README.md`: how to build the module without Clarity.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime module

The generated module carries no runtime sources. It requires the Go module `github.com/petervdpas/Clarity/runtimes/go`, which has these packages:

- `clarity`: records, storage (SQLite included), queries, migrations and the binding to the screen.
- `shell`: the Wails desktop shell.
- `postgresql`, `mysql` and `sqlserver`: one per server database. A generated module imports only the ones its data sources use, and its go.mod leaves out the others' dependencies.
- `engine`: runs [engines](code.engines.md) with wazero, imported only when the project has one.

The first build downloads them. To build on the runtime in Clarity's repository instead (while you work on the runtime itself), make a workspace in the output folder:

```shell
go work init .
go work edit -replace github.com/petervdpas/Clarity/runtimes/go=<Clarity>/runtimes/go
```

Clarity keeps `go.work` when it writes the module again. Remove it to build on the published module.

## What to install

- Go, from https://go.dev/dl
- On Linux, the WebKitGTK development files and pkg-config (Debian and Ubuntu: `libwebkit2gtk-4.1-dev`)

Clarity checks for these itself: run `clarity tools`, or look at Build tools on the Project tab. A first build compiles WebKit's bindings and can take several minutes.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder:

```shell
go build -tags production -trimpath -ldflags="-s -w" -o bin/<name> .
go run .
```

The program is `bin/<name>` (with `.exe` on Windows), where `<name>` is the module name.

## Windows on screen

The application has one desktop window. `open` shows a window in place, on top of the one that opened it, and `close` goes back to the one below. A dialog opens on top of its window in the same desktop window. A click target marked `in new window` opens a second Wails window. See [Windows](design.windows.md).

## The same on every stack

The screen is shared by the Rust, Go, C and C++ stacks, so they cannot differ; a window drawn from its markup draws exactly what the same window drawn from the model does, which a test checks for every window of every example. The parts of a window behave alike on the web renderer and on C# ([C# with Clarity.UI](stacks.csharp.md)): the shared cases in `conformance/` say what each part must do, and every renderer runs them.
