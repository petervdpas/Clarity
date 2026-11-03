# C# with Clarity.UI

How Clarity turns a project into a C# program whose windows Clarity.UI draws, on the desktop or in a browser, what you need to install, and how to build it.

Clarity.UI is Clarity's own UI framework: one library, written in Rust, that draws the windows itself with Skia, on the desktop and in a browser page, for every language (DESIGN 7.9). It draws every part of the catalog, the app frame (menu bar, toolbar, ribbon, sidebars, footer and status), dialogs, questions and the keys window, and plays sound. A C# program answers it from its view models.

## Choosing the stack

Name the stack in the project file, or pick it on the Project tab:

```clarity
stack csharp desktop
```

Clarity.UI is not named in the stack line: after the language come the targets, `desktop`, `web` or both. See [The project file](project.file.md).

## What is generated

Clarity writes a .NET project into the output folder (`out` by default). `<Name>` is the project name in Pascal case.

- `<Name>.csproj`: the project, targeting .NET 10 (`net10.0`).
- `Modules/<Module>.cs`: one file per Clarity module, with its enums, records, functions, actions and data events.
- `Windows/<Window>ViewModel.cs`: each window's view model, which holds its data, the state of its parts and its handlers.
- `Windows/<Window>.cxaml` and `Apps/<App>.cxaml`: each window and each app frame as markup, which Clarity.UI draws (see [Windows](design.windows.md)).
- `Program.cs`: the start of the program. It starts the language and theme kept from the last run, opens the databases and shows the main window.
- `Labels.cs`: the labels of enum values, for texts with slots.
- `Schema.cs`: the migration steps of every database Clarity owns.
- `Texts.resx` with the texts as written, `Texts.<language>.resx` per language, and `Texts.cs`, a typed class with a property per text. See [Languages](languages.md).
- `Themes/<theme>.light.json` and `Themes/<theme>.dark.json` per offered theme, as Clarity resolves it, and `AppThemes.cs`, which lists them. See [Themes](themes.md).
- `resources/`: the project's resources (pictures and Markdown files), embedded in the program.
- `README.md`: how to build the project without Clarity.

Each SQLite database opens in the folder the program keeps its data in. Set the environment variable `CLARITY_DATA` to another folder to run the program on other data, such as a copy, without changing the program.

### The web target

With `stack csharp web` the folder `out/web/` holds two projects and the page:

- `App`: the application, with the same view models, window markup, model, modules, texts and themes as the desktop program, without `Program.cs`. It also carries the files of `browser/` for the server.
- `Server`: the web server. It keeps the windows' view models and runs their handlers against the databases, as the desktop program does, and serves the page, Clarity.UI's browser files and the program's files.
- `browser/`: the page, and Clarity.UI's browser files (`clarity-ui.js`, `clarity-ui.wasm`, `clarity-ui-worker.js`, `clarity-ui-page.js`): the library compiled to WebAssembly once, the same for every language. Clarity puts them here when it writes the folder, from its installer or its release; `CLARITY_UI_BROWSER` names a folder of your own instead.

In the browser Clarity.UI draws the windows on the page's canvas and asks the server for everything a window does (`POST /clarity/invoke`), as the native library asks the program on the desktop. No handler, query or database connection runs in the browser. Dialogs are drawn over the page, a saved file is a download, a file to open comes from the page's file picker, and the person's choices stay in the browser. Each browser's windows have their own long call, with the bar and Stop, and windows a browser left for half an hour are let go.

Do not edit these files. Change the project in Clarity; it writes the folder again.

## The runtime packages

The generated project carries no runtime sources. It references NuGet packages from nuget.org, all at one shared version:

- `Clarity.Ui`: Clarity.UI, which draws the windows and runs them in the system's own window, on Linux, Windows and macOS. It carries Clarity.UI's native library for each system (Linux, Windows and macOS, each on x64 and arm64), and .NET loads the one for the system the program runs on. It brings `Clarity.Runtime`: the window model (view models, part state, undo), records, queries and migrations, without a database driver.
- `Clarity.Ui.Server`: the web target's server. It takes `Clarity.Runtime` alone, not Clarity.UI's native library: the browser draws.
- `Clarity.Runtime.Sqlite`, `Clarity.Runtime.PostgreSql`, `Clarity.Runtime.MySql` and `Clarity.Runtime.SqlServer`: one per kind of database, added only when a data source uses it.
- `Clarity.Runtime.Engines`: runs [engines](code.engines.md) with Wasmtime, added only when the project has one.

To build on the runtime's own projects in Clarity's repository instead (while you work on the runtime itself), name their folder:

```shell
dotnet build -p:ClarityRuntime=<Clarity>/runtimes/csharp
```

Or set the environment variable `CLARITY_RUNTIME` to that folder. Without either, the build uses the packages.

That route needs Clarity.UI's native library built in the repository first: `cargo build -p clarity-ui-ffi` for a Debug build of the program, `cargo build --release -p clarity-ui-ffi` for a Release build. The build puts it next to the program.

## What to install

- The .NET 10 SDK, from https://dotnet.microsoft.com/download

The web target needs nothing more: its browser files come with Clarity. Clarity checks for it itself: run `clarity tools`, or look at Build tools on the Project tab.

Step by step, with the commands for Windows, Linux and macOS: [Setting up the build tools](setup.tools.md).

## Build and run

**Generate** on the Preview tab writes the source into the output folder; then **Run** builds a debug program from it and starts it, and **Build** makes a release program. Without Clarity, in the output folder:

```shell
dotnet build -c Release
dotnet run
```

The release program is `bin/Release/net10.0/<Name>` (with `.exe` on Windows). A debug build goes to `bin/Debug/net10.0/<Name>`.

For the web target, build the server in `out/web/`, which builds the application it serves. Then start it and open its address in a browser:

```shell
dotnet build Server
dotnet Server/bin/Debug/net10.0/<Name>.Server.dll
```

It listens at the project's port (8080 unless the project file says otherwise); `--address host:port` or the environment variable `CLARITY_ADDRESS` makes it listen elsewhere. The server sends the browser files compressed to a browser that takes them. In Clarity, **Run** does all of this and the **Browser** button opens the address.

## Custom parts

A custom part on C# is a control of Clarity.UI, written in C# with any .NET package or assembly, through an adapter. See [Custom parts](design.custom-parts.md). It runs on the desktop; a program served to a browser alone has no custom parts, on any stack, since Clarity.UI runs alone in the browser.

## Not yet on C#

- An engine beside the screen in the browser: on the web target, engines run on the server, each frame a request.

## The same on every stack

The parts of a window behave alike on every stack. The shared cases in `conformance/` say what each part must do, and every version of Clarity.UI runs them.
